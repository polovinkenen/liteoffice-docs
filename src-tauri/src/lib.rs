mod commands;
mod menu;
mod open_files;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{Emitter, Manager, RunEvent};

#[derive(Default)]
pub struct AppState {
    pub dirty: AtomicBool,
    pub allow_exit: AtomicBool,
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "liteoffice_docs=info".into()),
        )
        .init();

    let app = tauri::Builder::default()
        .manage(AppState::default())
        .manage(open_files::PendingOpenFiles::default())
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            let cwd = if cwd.is_empty() {
                std::env::current_dir().unwrap_or_default()
            } else {
                PathBuf::from(cwd)
            };
            let paths = open_files::collect_paths(args.into_iter().skip(1), &cwd);
            open_files::queue(app, paths);
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let cwd = std::env::current_dir().unwrap_or_default();
            let initial = open_files::collect_paths(std::env::args_os().skip(1), &cwd);
            open_files::queue(app.handle(), initial);

            let app_menu = menu::build_menu(app.handle())?;
            app.set_menu(app_menu)?;
            app.on_menu_event(menu::handle_menu_event);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::app_info,
            commands::validate_document,
            commands::set_document_dirty,
            commands::quit_app,
            commands::document::open_document,
            commands::document::save_document,
            commands::autosave::autosave_document,
            commands::autosave::list_autosaves,
            commands::autosave::restore_autosave,
            commands::autosave::discard_autosave,
            commands::autosave::clear_autosaves,
            commands::settings::load_settings,
            commands::settings::save_settings,
            commands::recent::list_recent,
            commands::recent::add_recent,
            commands::recent::remove_recent,
            commands::recent::clear_recent,
            commands::image::import_image,
            commands::image::load_asset,
            commands::pdf::export_pdf,
            commands::pdf::open_pdf_preview,
            open_files::take_opened_paths
        ])
        .build(tauri::generate_context!())
        .expect("не удалось собрать LiteOffice Docs");

    app.run(|app_handle, event| match event {
        #[cfg(target_os = "macos")]
        RunEvent::Opened { urls } => {
            open_files::queue(app_handle, open_files::collect_urls(&urls));
        }
        RunEvent::ExitRequested { api, code, .. } => {
            let state = app_handle.state::<AppState>();
            let user_requested = code.is_none();

            if user_requested
                && state.dirty.load(Ordering::SeqCst)
                && !state.allow_exit.load(Ordering::SeqCst)
            {
                api.prevent_exit();
                let _ = app_handle.emit(menu::EVENT_APP_QUIT_REQUESTED, ());
            }
        }
        _ => {}
    });
}
