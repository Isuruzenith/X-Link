use tauri::State;

#[tauri::command]
pub fn get_buffered_logs(
    state: State<'_, crate::state::ProxyState>,
) -> Result<Vec<String>, String> {
    Ok(state.drain_logs())
}

/// Returns a clone of all current log lines from the ring buffer without draining them.
#[tauri::command]
pub fn get_logs(state: State<'_, crate::state::ProxyState>) -> Result<Vec<String>, String> {
    if let Ok(guard) = state.log_buffer.lock() {
        Ok(guard.iter().cloned().collect())
    } else {
        Ok(Vec::new())
    }
}

/// Clears the in-memory log buffer.
#[tauri::command]
pub fn clear_logs(state: State<'_, crate::state::ProxyState>) -> Result<(), String> {
    if let Ok(mut guard) = state.log_buffer.lock() {
        guard.clear();
    }
    Ok(())
}
