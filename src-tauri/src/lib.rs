mod commands;
mod project;
mod project_storage;
mod project_tree;
mod recurrence;
mod series;
mod series_storage;
mod settings;
mod settings_storage;
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
            let archive_dir = app_data_dir.join("archive");
            let projects_file = app_data_dir.join("projects.json");
            let settings_file = app_data_dir.join("settings.json");
            let series_file = app_data_dir.join("series.json");
            storage::migrate_scheduled_dates(&tasks_dir)?;

            let projects = project_storage::list_projects(&projects_file)?;
            let settings = settings_storage::load_settings(&settings_file)?;
            let projects = match commands::ensure_default_project(projects.clone(), settings) {
                Some((projects, settings)) => {
                    project_storage::save_projects(&projects_file, &projects)?;
                    settings_storage::save_settings(&settings_file, &settings)?;
                    projects
                }
                None => projects,
            };
            storage::migrate_task_project_names_to_ids(&tasks_dir, &projects)?;

            app.manage(commands::AppState {
                tasks_dir,
                archive_dir,
                projects_file,
                settings_file,
                series_file,
                projects_lock: std::sync::Mutex::new(()),
                series_lock: std::sync::Mutex::new(()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_tasks,
            commands::create_task,
            commands::create_recurring_task,
            commands::ensure_occurrences_until,
            commands::update_task,
            commands::update_series_occurrence,
            commands::delete_task,
            commands::delete_series_occurrence,
            commands::remove_recurrence,
            commands::get_series,
            commands::update_series_recurrence,
            commands::reorder_task,
            commands::list_projects,
            commands::create_project,
            commands::update_project,
            commands::delete_project,
            commands::ensure_subtask_container,
            commands::delete_subtask_container,
            commands::get_settings,
            commands::save_settings,
            commands::count_tasks_by_priority,
            commands::count_tasks_by_status,
            commands::finish_day
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
