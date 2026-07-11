use crate::memory::core_memory::CoreMemory;
use crate::relationship::{self, Milestone, RelationshipState};
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipInfo {
    pub state: RelationshipState,
    pub milestones: Vec<Milestone>,
    pub core_memory: CoreMemory,
}

#[tauri::command]
pub fn get_relationship(state: State<AppState>, persona_id: String) -> Result<RelationshipInfo, String> {
    let rel = relationship::get_or_create(&state.db, &persona_id);
    let milestones = relationship::list_milestones(&state.db, &persona_id);
    let core_mem = crate::memory::core_memory::get_or_create(&state.db, &persona_id);

    Ok(RelationshipInfo {
        state: rel,
        milestones,
        core_memory: core_mem,
    })
}

#[tauri::command]
pub fn get_all_relationships(state: State<AppState>) -> Result<Vec<RelationshipState>, String> {
    let personas = crate::storage::repo::list_personas(&state.db)
        .map_err(|e| e.to_string())?;
    let mut rels = Vec::new();
    for p in &personas {
        rels.push(relationship::get_or_create(&state.db, &p.id));
    }
    Ok(rels)
}

#[tauri::command]
pub fn update_core_memory(state: State<AppState>, persona_id: String, memory: CoreMemory) -> Result<(), String> {
    let mut mem = memory;
    mem.persona_id = persona_id.clone();
    crate::memory::core_memory::save(&state.db, &mem)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_nickname(state: State<AppState>, persona_id: String, nickname: String) -> Result<RelationshipState, String> {
    let mut rel = relationship::get_or_create(&state.db, &persona_id);
    let new_ms = relationship::record_interaction(
        &state.db,
        &mut rel,
        relationship::InteractionEvent::NicknameSet(nickname),
    );
    for ms in new_ms {
        log::info!("Milestone achieved: {} - {}", ms.title, ms.description);
    }
    Ok(rel)
}

#[tauri::command]
pub fn reset_relationship(state: State<AppState>, persona_id: String) -> Result<(), String> {
    state.db.with_conn(|conn| {
        conn.execute("DELETE FROM relationship WHERE persona_id = ?1", [&persona_id])?;
        conn.execute("DELETE FROM milestone WHERE persona_id = ?1", [&persona_id])?;
        conn.execute("DELETE FROM core_memory WHERE persona_id = ?1", [&persona_id])?;
        Ok::<_, rusqlite::Error>(())
    }).map_err(|e| e.to_string())?;
    Ok(())
}
