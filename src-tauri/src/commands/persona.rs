use crate::state::AppState;
use crate::storage::models::{PersonaDefinition, PersonaRow};
use crate::storage::repo;
use tauri::State;

#[tauri::command]
pub fn list_personas(state: State<'_, AppState>) -> Result<Vec<PersonaRow>, String> {
    repo::list_personas(&state.db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_persona(state: State<'_, AppState>, id: String) -> Result<Option<PersonaRow>, String> {
    let mut row = repo::get_persona(&state.db, &id).map_err(|e| e.to_string())?;
    if let Some(ref mut r) = row {
        let workflow_rows = repo::list_workflows(&state.db, &r.id).map_err(|e| e.to_string())?;
        let workflows: Result<Vec<_>, _> = workflow_rows
            .iter()
            .map(repo::workflow_row_to_workflow)
            .collect();
        r.definition.workflows = Some(workflows?);
    }
    Ok(row)
}

#[tauri::command]
pub fn update_persona(
    state: State<'_, AppState>,
    mut persona: PersonaDefinition,
) -> Result<(), String> {
    // 工作流通过独立接口维护，避免重复写入 definition_json
    persona.workflows = None;
    repo::update_persona(&state.db, &persona).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_persona(state: State<'_, AppState>, id: String) -> Result<(), String> {
    repo::set_active_persona(&state.db, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_active_persona_id(state: State<'_, AppState>) -> Result<Option<String>, String> {
    repo::get_active_persona_id(&state.db).map_err(|e| e.to_string())
}
