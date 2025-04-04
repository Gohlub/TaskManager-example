use hyperware_app_common::{Binding, SaveOptions, SendResult};
use hyperware_process_lib::http::server::{HttpBindingConfig, WsBindingConfig, WsMessageType};
use hyperware_process_lib::{Address, LazyLoadBlob};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Import caller utilities after running hyper-bindgen
use caller_utils::task_storage::{add_task_remote_rpc, get_tasks_by_status_remote_rpc};

// Define task-related types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    id: String,
    title: String,
    description: String,
    status: TaskStatus,
    created_at: u64,
    assigned_to: Option<String>,
}

// Define application state
#[derive(Default, Debug, Serialize, Deserialize)]
struct TaskManagerState {
    // In-memory task storage
    tasks: HashMap<String, Task>,
    
    // Track active WebSocket connections for real-time updates
    active_ws_connections: HashMap<u32, String>, // channel_id -> client_id
    
    // Analytics
    request_count: u64,
    task_creation_count: u64,
}

// Implement the application logic
#[hyperprocess(
    name = "Task Manager",
    icon = "task-icon",
    widget = "task-widget",
    ui = Some(HttpBindingConfig::new(true, true, false, None)),
    endpoints = vec![
        // Main API endpoint
        Binding::Http {
            path: "/api/tasks",
            config: HttpBindingConfig::new(false, false, false, None)
        },
        // WebSocket for real-time updates
        Binding::Ws {
            path: "/ws/tasks",
            config: WsBindingConfig::new(false, false, false)
        }
    ],
    save_config = SaveOptions::EveryNMessage(5),
    wit_world = "task-manager-dot-os-v0"
)]
impl TaskManagerState {
    /// Initialize the process on startup
    #[init]
    async fn initialize(&mut self) {
        // Simulate loading some initial data
        let default_task = Task {
            id: Uuid::new_v4().to_string(),
            title: "Welcome Task".to_string(),
            description: "This is your first task!".to_string(),
            status: TaskStatus::Pending,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            assigned_to: None,
        };
        
        self.tasks.insert(default_task.id.clone(), default_task);
        
        // Perform any async initialization with other processes
        match get_stored_tasks().await {
            Ok(stored_tasks) => {
                for task in stored_tasks {
                    self.tasks.insert(task.id.clone(), task);
                }
                hyperware_process_lib::logging::info!("Loaded {} tasks from storage", stored_tasks.len());
            }
            Err(e) => {
                hyperware_process_lib::logging::warn!("Failed to load tasks from storage: {:?}", e);
            }
        }
    }
    
    /// Create a new task via HTTP endpoint
    #[http]
    async fn create_task(&mut self, new_task_req: NewTaskRequest) -> TaskResponse {
        self.request_count += 1;
        
        // Generate new task with UUID
        let task_id = Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: new_task_req.title,
            description: new_task_req.description,
            status: TaskStatus::Pending,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            assigned_to: new_task_req.assigned_to,
        };
        
        // Store task locally
        self.tasks.insert(task_id.clone(), task.clone());
        self.task_creation_count += 1;
        
        // Asynchronously store in the persistent storage process
        let storage_result = store_task_in_storage(&task).await;
        
        // Notify connected WebSocket clients about the new task
        self.broadcast_task_update(&task);
        
        // Return response with task info and storage status
        TaskResponse {
            success: true,
            task: Some(task),
            storage_status: storage_result.is_ok(),
            message: "Task created successfully".to_string(),
        }
    }
    
    /// Get a list of all tasks via HTTP endpoint
    #[http]
    fn get_all_tasks(&mut self) -> Vec<Task> {
        self.request_count += 1;
        self.tasks.values().cloned().collect()
    }
    
    /// Get a specific task by ID via HTTP endpoint
    #[http]
    fn get_task(&mut self, task_id: String) -> TaskResponse {
        self.request_count += 1;
        
        match self.tasks.get(&task_id) {
            Some(task) => TaskResponse {
                success: true,
                task: Some(task.clone()),
                storage_status: true,
                message: "Task found".to_string(),
            },
            None => TaskResponse {
                success: false,
                task: None,
                storage_status: true,
                message: "Task not found".to_string(),
            },
        }
    }
    
    /// Update a task's status via HTTP endpoint
    #[http]
    async fn update_task_status(&mut self, update_req: TaskStatusUpdateRequest) -> TaskResponse {
        self.request_count += 1;
        
        if let Some(task) = self.tasks.get_mut(&update_req.task_id) {
            task.status = update_req.new_status;
            
            // Store updated task in storage
            let storage_result = store_task_in_storage(task).await;
            
            // Notify connected clients
            self.broadcast_task_update(task);
            
            TaskResponse {
                success: true,
                task: Some(task.clone()),
                storage_status: storage_result.is_ok(),
                message: "Task updated successfully".to_string(),
            }
        } else {
            TaskResponse {
                success: false,
                task: None,
                storage_status: false,
                message: "Task not found".to_string(),
            }
        }
    }
    
    /// Handle local request to get task statistics
    #[local]
    fn get_statistics(&mut self) -> TaskManagerStats {
        TaskManagerStats {
            total_tasks: self.tasks.len() as u64,
            pending_tasks: self.tasks.values().filter(|t| matches!(t.status, TaskStatus::Pending)).count() as u64,
            completed_tasks: self.tasks.values().filter(|t| matches!(t.status, TaskStatus::Completed)).count() as u64,
            creation_count: self.task_creation_count,
            request_count: self.request_count,
        }
    }
    
    /// Handle both local and remote requests to get tasks by status
    #[local]
    #[remote]
    fn get_tasks_by_status(&mut self, status: TaskStatus) -> Vec<Task> {
        self.tasks
            .values()
            .filter(|task| task.status == status)
            .cloned()
            .collect()
    }
    
    /// Handle WebSocket messages for real-time updates
    #[ws]
    fn handle_websocket(&mut self, channel_id: u32, message_type: WsMessageType, blob: LazyLoadBlob) {
        match message_type {
            WsMessageType::Binary => {
                // Handle binary message (example: could be task updates from clients)
                if let Ok(ws_message) = serde_json::from_slice::<WebSocketMessage>(blob.bytes()) {
                    match ws_message {
                        WebSocketMessage::Subscribe { client_id } => {
                            // Register client for updates
                            self.active_ws_connections.insert(channel_id, client_id);
                            
                            // Send current tasks as initial data
                            if let Some(server) = hyperware_app_common::get_server() {
                                let tasks = self.get_all_tasks();
                                if let Ok(tasks_json) = serde_json::to_vec(&tasks) {
                                    let _ = server.send_ws_message(channel_id, WsMessageType::Binary, tasks_json);
                                }
                            }
                        }
                        WebSocketMessage::Unsubscribe => {
                            // Remove client subscription
                            self.active_ws_connections.remove(&channel_id);
                        }
                    }
                }
            }
            WsMessageType::Close => {
                // Client disconnected, remove from active connections
                self.active_ws_connections.remove(&channel_id);
            }
            _ => { /* Ignore other message types */ }
        }
    }
    
    // Helper method to broadcast updates to all connected WebSocket clients
    fn broadcast_task_update(&self, task: &Task) {
        if let Some(server) = hyperware_app_common::get_server() {
            if let Ok(task_json) = serde_json::to_vec(&task) {
                for channel_id in self.active_ws_connections.keys() {
                    let _ = server.send_ws_message(*channel_id, WsMessageType::Binary, task_json.clone());
                }
            }
        }
    }
}

// Supporting types for the application
#[derive(Debug, Serialize, Deserialize)]
struct NewTaskRequest {
    title: String,
    description: String,
    assigned_to: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskStatusUpdateRequest {
    task_id: String,
    new_status: TaskStatus,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskResponse {
    success: bool,
    task: Option<Task>,
    storage_status: bool,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskManagerStats {
    total_tasks: u64,
    pending_tasks: u64,
    completed_tasks: u64,
    creation_count: u64,
    request_count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
enum WebSocketMessage {
    Subscribe { client_id: String },
    Unsubscribe,
}

// Helper functions for communicating with other processes
async fn store_task_in_storage(task: &Task) -> SendResult<bool> {
    // Get the address of the storage process
    let storage_addr = Address::process("task-storage:app:sys");
    
    // Call the remote function to store the task
    add_task_remote_rpc(&storage_addr, task.clone(), 5).await
}

async fn get_stored_tasks() -> Result<Vec<Task>, String> {
    // Get the address of the storage process
    let storage_addr = Address::process("task-storage:app:sys");
    
    // Call the remote function to get tasks
    match get_tasks_by_status_remote_rpc(&storage_addr, TaskStatus::Pending, 5).await {
        SendResult::Success(tasks) => Ok(tasks),
        SendResult::Timeout => Err("Timeout connecting to storage".to_string()),
        SendResult::Offline => Err("Storage service is offline".to_string()),
        SendResult::DeserializationError(e) => Err(format!("Failed to deserialize tasks: {}", e)),
    }
}