use rustyline::DefaultEditor;

/// Returns the path for the interactive history file.
fn history_path() -> Option<std::path::PathBuf> {
    dirs::data_local_dir().map(|d| d.join("pplx").join("history"))
}

/// Create a rustyline editor and load history.
pub fn create_editor() -> DefaultEditor {
    let mut rl = DefaultEditor::new().unwrap_or_else(|_| {
        // Fall back to default if config fails
        DefaultEditor::new().expect("Failed to create line editor")
    });
    load_history(&mut rl);
    rl
}

/// Load history from the persistent file.
pub fn load_history(rl: &mut DefaultEditor) {
    if let Some(path) = history_path() {
        let _ = rl.load_history(&path);
    }
}

/// Save history to the persistent file, creating the directory if needed.
pub fn save_history(rl: &mut DefaultEditor) {
    if let Some(path) = history_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = rl.save_history(&path);
    }
}
