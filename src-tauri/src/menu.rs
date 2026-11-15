use tauri::menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub const FILE_NEW: &str = "file.new";
pub const FILE_OPEN: &str = "file.open";
pub const FILE_SAVE: &str = "file.save";
pub const FILE_SAVE_AS: &str = "file.save-as";
pub const FILE_PRINT: &str = "file.print";
pub const FILE_CLOSE_DOCUMENT: &str = "file.close-document";
pub const FILE_CLOSE_WINDOW: &str = "file.close-window";
pub const APP_QUIT: &str = "app.quit";

pub const EVENT_FILE_NEW: &str = "menu:file:new";
pub const EVENT_FILE_OPEN: &str = "menu:file:open";
pub const EVENT_FILE_SAVE: &str = "menu:file:save";
pub const EVENT_FILE_SAVE_AS: &str = "menu:file:save-as";
pub const EVENT_FILE_CLOSE_DOCUMENT: &str = "menu:file:close-document";
pub const EVENT_FILE_CLOSE_WINDOW: &str = "menu:file:close-window";
pub const EVENT_FILE_PRINT_FALLBACK: &str = "menu:file:print-fallback";
pub const EVENT_APP_QUIT: &str = "menu:app:quit";
pub const EVENT_APP_QUIT_REQUESTED: &str = "menu:app:quit-requested";

pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let quit = MenuItemBuilder::with_id(APP_QUIT, "Выйти из Wordlite")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;

    let app_menu = build_app_submenu(app, &quit)?;

    let new_document = MenuItemBuilder::with_id(FILE_NEW, "Новый")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let open = MenuItemBuilder::with_id(FILE_OPEN, "Открыть…")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
    let save = MenuItemBuilder::with_id(FILE_SAVE, "Сохранить")
        .accelerator("CmdOrCtrl+S")
        .build(app)?;
    let save_as = MenuItemBuilder::with_id(FILE_SAVE_AS, "Сохранить как…")
        .accelerator("CmdOrCtrl+Shift+S")
        .build(app)?;
    let print = MenuItemBuilder::with_id(FILE_PRINT, "Печать…")
        .accelerator("CmdOrCtrl+P")
        .build(app)?;
    let close_document = MenuItemBuilder::with_id(FILE_CLOSE_DOCUMENT, "Закрыть документ")
        .accelerator("CmdOrCtrl+W")
        .build(app)?;
    let close_window = MenuItemBuilder::with_id(FILE_CLOSE_WINDOW, "Закрыть окно")
        .accelerator("CmdOrCtrl+Shift+W")
        .build(app)?;

    let file_menu = SubmenuBuilder::new(app, "Файл")
        .item(&new_document)
        .item(&open)
        .separator()
        .item(&save)
        .item(&save_as)
        .separator()
        .item(&print)
        .separator()
        .item(&close_document)
        .item(&close_window)
        .build()?;

    let window_menu = SubmenuBuilder::new(app, "Окно")
        .minimize_with_text("Свернуть")
        .maximize_with_text("Масштаб")
        .build()?;

    MenuBuilder::new(app)
        .items(&[&app_menu, &file_menu, &window_menu])
        .build()
}

fn build_app_submenu<R: Runtime>(
    app: &AppHandle<R>,
    quit: &tauri::menu::MenuItem<R>,
) -> tauri::Result<tauri::menu::Submenu<R>> {
    let submenu = SubmenuBuilder::new(app, "Wordlite").about_with_text("О Wordlite", None);

    #[cfg(target_os = "macos")]
    let submenu = submenu
        .separator()
        .services_with_text("Службы")
        .separator()
        .hide_with_text("Скрыть Wordlite")
        .hide_others_with_text("Скрыть остальные")
        .show_all_with_text("Показать все");

    submenu.separator().item(quit).build()
}

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    if event.id().as_ref() == FILE_PRINT {
        print_window(app);
        return;
    }

    let event_name = match event.id().as_ref() {
        FILE_NEW => EVENT_FILE_NEW,
        FILE_OPEN => EVENT_FILE_OPEN,
        FILE_SAVE => EVENT_FILE_SAVE,
        FILE_SAVE_AS => EVENT_FILE_SAVE_AS,
        FILE_CLOSE_DOCUMENT => EVENT_FILE_CLOSE_DOCUMENT,
        FILE_CLOSE_WINDOW => EVENT_FILE_CLOSE_WINDOW,
        APP_QUIT => EVENT_APP_QUIT,
        _ => return,
    };

    if let Err(error) = app.emit(event_name, ()) {
        tracing::warn!(%error, event = event_name, "не удалось отправить событие меню");
    }
}

fn print_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    if let Err(error) = window.print() {
        tracing::warn!(%error, "не удалось открыть диалог печати, используем fallback");
        let _ = app.emit(EVENT_FILE_PRINT_FALLBACK, ());
    }
}
