use tauri::State;

#[tauri::command]
pub fn get_buffered_logs(state: State<'_, crate::state::ProxyState>) -> Result<Vec<String>, String> {
    Ok(state.drain_logs())
}
