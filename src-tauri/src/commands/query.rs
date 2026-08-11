use crate::app_state::AppState;
use crate::llm::answer;

#[tauri::command]
pub fn query_memories(state: tauri::State<'_, AppState>, question: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    let ctx = crate::query::search_context(&conn, &question, 8)?;
    let llm = state.llm.clone();
    let answer = answer::answer_question(llm.as_ref(), &question, &ctx.hits, &ctx.people, &ctx.prefs)
        .map_err(|e| e.to_string())?;
    Ok(answer)
}
