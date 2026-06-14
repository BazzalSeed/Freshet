pub mod agent;
pub mod engine;
pub mod model;
pub mod scheduler;
pub mod store;
pub mod sources;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Fix the GUI-launch PATH so child processes (agents, source fetches)
    // inherit the user's shell PATH. Guarded out of tests so unit tests never
    // mutate the process environment.
    #[cfg(not(test))]
    {
        // UNVERIFIED: live path
        let _ = fix_path_env::fix();
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
