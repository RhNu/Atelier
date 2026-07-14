mod commands;
mod desktop;
mod desktop_system;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Runs the desktop shell.
///
/// # Panics
///
/// Panics if the Tauri runtime cannot start.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::desktop_paths,
            commands::read_clipboard_image,
            commands::pick_workspace_directory,
            commands::pick_export_directory,
            commands::pick_and_import_image_resources,
            commands::pick_and_import_vibe_documents,
            commands::pick_and_import_embedded_png_vibe_documents,
            commands::save_vibe_document,
            commands::save_resource_image,
            commands::open_path,
            commands::reveal_path,
            commands::open_workspace,
            commands::workspace_status,
            commands::close_workspace,
            commands::create_api_key,
            commands::update_api_key,
            commands::delete_api_key,
            commands::list_api_keys,
            commands::set_active_api_key,
            commands::probe_api_key,
            commands::probe_active_api_key,
            commands::cached_active_subscription,
            commands::upsert_prompt_chunk,
            commands::get_prompt_chunk,
            commands::list_prompt_chunks,
            commands::delete_prompt_chunk,
            commands::upsert_prompt_preset,
            commands::list_prompt_presets,
            commands::delete_prompt_preset,
            commands::compile_prompt_preview,
            commands::compile_generation_prompt_preview,
            commands::prompt_lexicon_catalog,
            commands::prompt_lexicon_list,
            commands::prompt_lexicon_search,
            commands::get_resource_image,
            commands::get_workspace_settings,
            commands::update_workspace_settings,
            commands::reset_workspace_settings,
            commands::submit_generation,
            commands::submit_generation_batch,
            commands::estimate_generation,
            commands::run_generation_job,
            commands::pause_generation_queue,
            commands::resume_generation_queue,
            commands::stop_generation_queue,
            commands::generation_delay_elapsed,
            commands::generation_status,
            commands::query_run_history,
            commands::delete_run_history_items,
            commands::rerun_generation_history_item,
            commands::run_director_tool,
            commands::ensure_vibe_encoding,
            commands::list_vibe_documents,
            commands::get_vibe_document,
            commands::rename_vibe_document,
            commands::set_vibe_document_hidden,
            commands::query_gallery,
            commands::set_gallery_safety_override,
            commands::delete_gallery_items,
            commands::gallery_image_reference,
            commands::events_since,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let state = desktop::build_desktop_state(app.handle().clone())?;
            app.manage(state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
