mod commands;
mod desktop;
mod desktop_system;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Runs the desktop shell.
///
/// # Panics
///
/// Panics if the Tauri runtime cannot start.
pub fn run() {
    let app = tauri::Builder::default()
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
            commands::save_resource_images_zip,
            commands::open_path,
            commands::reveal_path,
            commands::bootstrap_app,
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
            commands::release_imported_image_resources,
            commands::get_global_settings,
            commands::update_global_settings,
            commands::get_workspace_settings,
            commands::update_workspace_settings,
            commands::reset_workspace_settings,
            commands::get_generation_draft,
            commands::save_generation_draft,
            commands::clear_generation_draft,
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
            commands::query_generation_history,
            commands::get_generation_history_batch,
            commands::delete_run_history_items,
            commands::delete_generation_history_batches,
            commands::rerun_generation_history_item,
            commands::rerun_generation_history_batch,
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
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    let shutdown_started = Arc::new(AtomicBool::new(false));
    let shutdown_complete = Arc::new(AtomicBool::new(false));
    app.run(move |app_handle, event| {
        handle_run_event(app_handle, event, &shutdown_started, &shutdown_complete);
    });
}

fn handle_run_event(
    app_handle: &tauri::AppHandle,
    event: RunEvent,
    shutdown_started: &Arc<AtomicBool>,
    shutdown_complete: &Arc<AtomicBool>,
) {
    let RunEvent::ExitRequested { api, .. } = event else {
        return;
    };
    if shutdown_complete.load(Ordering::Acquire) {
        return;
    }

    api.prevent_exit();
    if shutdown_started.swap(true, Ordering::AcqRel) {
        return;
    }

    let app_handle = app_handle.clone();
    let shutdown_complete = shutdown_complete.clone();
    tauri::async_runtime::spawn(async move {
        app_handle.state::<desktop::DesktopState>().shutdown().await;
        shutdown_complete.store(true, Ordering::Release);
        app_handle.exit(0);
    });
}
