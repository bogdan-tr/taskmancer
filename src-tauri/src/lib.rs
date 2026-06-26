mod commands;
mod layout;
mod layout_storage;
mod project;
mod project_storage;
mod project_tree;
mod recurrence;
mod series;
mod series_storage;
mod settings;
mod settings_storage;
mod status_stats;
mod status_tier;
mod storage;
mod task;
mod time_storage;
mod time_tracking;

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
            let layouts_file = app_data_dir.join("layouts.json");
            let time_db_file = app_data_dir.join("time_tracking.sqlite");
            storage::migrate_scheduled_dates(&tasks_dir)?;

            std::fs::create_dir_all(&app_data_dir)?;
            let time_db = rusqlite::Connection::open(&time_db_file)?;
            time_storage::init_schema(&time_db)?;

            let projects = project_storage::list_projects(&projects_file)?;
            let settings = settings_storage::load_settings(&settings_file)?;
            let (projects, settings) =
                match commands::ensure_default_project(projects.clone(), settings.clone()) {
                    Some((projects, settings)) => {
                        project_storage::save_projects(&projects_file, &projects)?;
                        settings_storage::save_settings(&settings_file, &settings)?;
                        (projects, settings)
                    }
                    None => (projects, settings),
                };
            storage::migrate_task_project_names_to_ids(&tasks_dir, &projects)?;

            let layouts = layout_storage::list_layouts(&layouts_file)?;
            if let Some((layouts, settings)) =
                layout::ensure_default_status_line_layout(layouts, settings)
            {
                layout_storage::save_layouts(&layouts_file, &layouts)?;
                settings_storage::save_settings(&settings_file, &settings)?;
            }

            let layouts = layout_storage::list_layouts(&layouts_file)?;
            let settings = settings_storage::load_settings(&settings_file)?;
            if let Some((layouts, settings)) =
                layout::ensure_default_dashboard_layout(layouts, settings)
            {
                layout_storage::save_layouts(&layouts_file, &layouts)?;
                settings_storage::save_settings(&settings_file, &settings)?;
            }

            app.manage(commands::AppState {
                tasks_dir,
                archive_dir,
                projects_file,
                settings_file,
                series_file,
                layouts_file,
                projects_lock: std::sync::Mutex::new(()),
                series_lock: std::sync::Mutex::new(()),
                time_db: std::sync::Mutex::new(time_db),
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
            commands::finish_day,
            commands::start_tracking,
            commands::stop_tracking,
            commands::get_active_sessions,
            commands::heartbeat,
            commands::resolve_orphaned_session,
            commands::add_manual_time_entry,
            commands::update_time_entry,
            commands::delete_time_entry,
            commands::list_time_entries,
            commands::start_project_tracking,
            commands::stop_project_tracking,
            commands::get_project_status_stats,
            commands::get_global_status_stats,
            commands::list_status_layouts,
            commands::create_status_layout,
            commands::update_status_layout,
            commands::duplicate_status_layout,
            commands::delete_status_layout,
            commands::get_dashboard_time_by_project,
            commands::get_dashboard_time_by_tag,
            commands::get_dashboard_estimated_vs_actual,
            commands::get_dashboard_completion_trend,
            commands::get_dashboard_status_distribution,
            commands::get_dashboard_busy_histogram
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
