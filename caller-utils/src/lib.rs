wit_bindgen::generate!({
    path: "target/wit",
    world: "types-task-manager-dot-os-v0",
    generate_unused_types: true,
    additional_derives: [serde::Deserialize, serde::Serialize, process_macros::SerdeJsonInto],
});

/// Generated caller utilities for RPC function stubs

pub use hyperware_app_common::SendResult;
pub use hyperware_app_common::send;
use hyperware_process_lib::Address;
use serde_json::json;

// Import specific types from each interface
pub use crate::wit_custom::TaskStatus;
pub use crate::wit_custom::Task;
pub use crate::wit_custom::TaskManagerStats;
pub use crate::wit_custom::TaskStatusUpdateRequest;
pub use crate::wit_custom::TaskResponse;
pub use crate::wit_custom::NewTaskRequest;
pub use crate::wit_custom::TaskStatus;
pub use crate::wit_custom::Task;
pub use crate::wit_custom::TaskManagerStats;
pub use crate::wit_custom::TaskStatusUpdateRequest;
pub use crate::wit_custom::TaskResponse;
pub use crate::wit_custom::NewTaskRequest;

/// Generated RPC stubs for the task_manager interface
pub mod task_manager {
    use crate::*;

    /// Generated stub for `create-task` http RPC call
    pub async fn create_task_http_rpc(_target: &str, _new_task_req:  NewTaskRequest) -> SendResult<TaskResponse> {
        // TODO: Implement HTTP endpoint
        SendResult::Success(TaskResponse::default())
    }
    
    /// Generated stub for `get-all-tasks` http RPC call
    pub async fn get_all_tasks_http_rpc(_target: &str) -> SendResult<Vec<Task>> {
        // TODO: Implement HTTP endpoint
        SendResult::Success(Vec::new())
    }
    
    /// Generated stub for `get-task` http RPC call
    pub async fn get_task_http_rpc(_target: &str, _task_id:  String) -> SendResult<TaskResponse> {
        // TODO: Implement HTTP endpoint
        SendResult::Success(TaskResponse::default())
    }
    
    /// Generated stub for `update-task-status` http RPC call
    pub async fn update_task_status_http_rpc(_target: &str, _update_req:  TaskStatusUpdateRequest) -> SendResult<TaskResponse> {
        // TODO: Implement HTTP endpoint
        SendResult::Success(TaskResponse::default())
    }
    
    /// Generated stub for `get-statistics` local RPC call
    pub async fn get_statistics_local_rpc(target: &Address) -> SendResult<TaskManagerStats> {
        let request = json!({"GetStatistics" : {}});
        send::<TaskManagerStats>(&request, target, 30).await
    }
    
    /// Generated stub for `get-tasks-by-status` remote RPC call
    pub async fn get_tasks_by_status_remote_rpc(target: &Address, status: TaskStatus) -> SendResult<Vec<Task>> {
        let request = json!({"GetTasksByStatus": status});
        send::<Vec<Task>>(&request, target, 30).await
    }
    
    /// Generated stub for `get-tasks-by-status` local RPC call
    pub async fn get_tasks_by_status_local_rpc(target: &Address, status: TaskStatus) -> SendResult<Vec<Task>> {
        let request = json!({"GetTasksByStatus": status});
        send::<Vec<Task>>(&request, target, 30).await
    }
    
    
}

