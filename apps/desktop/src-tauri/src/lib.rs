mod commands;
mod desktop;
mod desktop_system;
mod shutdown;

use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Runs the desktop shell.
///
/// # Panics
///
/// Panics if the Tauri runtime cannot start.
#[allow(
    clippy::too_many_lines,
    reason = "the Tauri command registry is intentionally centralized"
)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(desktop_log_level())
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::desktop_paths,
            commands::read_clipboard_image,
            commands::copy_text_to_clipboard,
            commands::open_external_url,
            commands::import_clipboard_image_resource,
            commands::pick_workspace_directory,
            commands::pick_export_directory,
            commands::pick_and_import_image_resources,
            commands::pick_and_import_vibe_documents,
            commands::pick_and_import_embedded_png_vibe_documents,
            commands::save_vibe_document,
            commands::save_resource_image,
            commands::copy_resource_image,
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
            commands::lexicon_bootstrap,
            commands::lexicon_complete,
            commands::lexicon_search,
            commands::lexicon_entity,
            commands::get_resource_image,
            commands::release_imported_image_resources,
            commands::get_global_settings,
            commands::update_global_settings,
            commands::set_notification_language,
            commands::get_danbooru_account,
            commands::save_danbooru_account,
            commands::probe_danbooru_account,
            commands::delete_danbooru_account,
            commands::search_danbooru_posts,
            commands::get_danbooru_post_detail,
            commands::get_danbooru_media,
            commands::list_downloadable_resources,
            commands::refresh_downloadable_resource_catalog,
            commands::complete_downloadable_resource_onboarding,
            commands::install_downloadable_resource,
            commands::install_downloadable_resource_group,
            commands::cancel_downloadable_resource_install,
            commands::delete_downloadable_resource,
            commands::check_app_update,
            commands::install_app_update,
            commands::get_workspace_settings,
            commands::update_workspace_settings,
            commands::reset_workspace_settings,
            commands::get_generation_draft,
            commands::save_generation_draft,
            commands::clear_generation_draft,
            commands::append_lexicon_entities_to_generation_draft,
            commands::submit_generation,
            commands::submit_generation_batch,
            commands::estimate_generation,
            commands::list_image_models,
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
            commands::get_gallery_item_detail,
            commands::set_gallery_safety_override,
            commands::rescan_gallery_safety,
            commands::delete_gallery_items,
            commands::gallery_image_reference,
            commands::events_since,
        ])
        .setup(|app| {
            let state = desktop::build_desktop_state(app.handle().clone())?;
            app.manage(state);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    let shutdown = shutdown::ShutdownCoordinator::default();
    app.run(move |app_handle, event| {
        handle_run_event(app_handle, event, &shutdown);
    });
}

const fn desktop_log_level() -> log::LevelFilter {
    if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    }
}

fn handle_run_event(
    app_handle: &tauri::AppHandle,
    event: RunEvent,
    shutdown: &shutdown::ShutdownCoordinator,
) {
    let RunEvent::ExitRequested { api, .. } = event else {
        return;
    };
    match shutdown.on_exit_requested() {
        shutdown::ExitRequestAction::BeginShutdown => {
            api.prevent_exit();
            shutdown.begin(app_handle);
        }
        shutdown::ExitRequestAction::WaitForShutdown => api.prevent_exit(),
        shutdown::ExitRequestAction::AllowExit => {}
    }
}
