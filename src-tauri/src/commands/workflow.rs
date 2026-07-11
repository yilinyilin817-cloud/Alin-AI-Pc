use crate::state::AppState;
use crate::storage::models::WorkflowRow;
use crate::storage::repo;
use tauri::State;

#[tauri::command]
pub fn list_workflows(
    persona_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<WorkflowRow>, String> {
    repo::list_workflows(&state.db, &persona_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_workflow(
    workflow_id: String,
    state: State<'_, AppState>,
) -> Result<Option<WorkflowRow>, String> {
    repo::get_workflow(&state.db, &workflow_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_workflow(
    payload: WorkflowRow,
    state: State<'_, AppState>,
) -> Result<WorkflowRow, String> {
    repo::insert_workflow(&state.db, &payload).map_err(|e| e.to_string())?;
    repo::get_workflow(&state.db, &payload.id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "workflow not found after create".into())
}

#[tauri::command]
pub fn update_workflow(
    payload: WorkflowRow,
    state: State<'_, AppState>,
) -> Result<WorkflowRow, String> {
    repo::update_workflow(&state.db, &payload).map_err(|e| e.to_string())?;
    repo::get_workflow(&state.db, &payload.id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "workflow not found after update".into())
}

#[tauri::command]
pub fn delete_workflow(workflow_id: String, state: State<'_, AppState>) -> Result<(), String> {
    repo::delete_workflow(&state.db, &workflow_id).map_err(|e| e.to_string())
}
