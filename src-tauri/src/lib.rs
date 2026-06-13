mod commands;
mod project;
mod project_storage;
mod storage;
mod task;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let tasks_dir = app_data_dir.join("tasks");
            let projects_file = app_data_dir.join("projects.json");
            app.manage(commands::AppState {
                tasks_dir,
                projects_file,
                projects_lock: std::sync::Mutex::new(()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_tasks,
            commands::create_task,
            commands::update_task_status,
            commands::update_task,
            commands::delete_task,
            commands::reorder_task,
            commands::list_projects,
            commands::create_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
