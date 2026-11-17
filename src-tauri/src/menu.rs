use std::path::Path;

use tauri::menu::{
    Menu, MenuBuilder, MenuEvent, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use wlite_app_core::recent::{RecentEntry, RecentStore};

pub const RECENT_PREFIX: &str = "recent:";
pub const RECENT_CLEAR: &str = "recent:clear";
pub const RECENT_EMPTY: &str = "recent:empty";

pub const FILE_NEW: &str = "file.new";
pub const FILE_OPEN: &str = "file.open";
pub const FILE_SAVE: &str = "file.save";
pub const FILE_SAVE_AS: &str = "file.save-as";
pub const FILE_PRINT: &str = "file.print";
pub const FILE_CLOSE_DOCUMENT: &str = "file.close-document";
pub const FILE_CLOSE_WINDOW: &str = "file.close-window";
pub const EDIT_UNDO: &str = "edit.undo";
pub const EDIT_REDO: &str = "edit.redo";
pub const APP_QUIT: &str = "app.quit";

pub const EVENT_FILE_NEW: &str = "menu:file:new";
pub const EVENT_FILE_OPEN: &str = "menu:file:open";
pub const EVENT_FILE_SAVE: &str = "menu:file:save";
pub const EVENT_FILE_SAVE_AS: &str = "menu:file:save-as";
pub const EVENT_FILE_CLOSE_DOCUMENT: &str = "menu:file:close-document";
pub const EVENT_FILE_CLOSE_WINDOW: &str = "menu:file:close-window";
pub const EVENT_FILE_PRINT_FALLBACK: &str = "menu:file:print-fallback";
pub const EVENT_EDIT_UNDO: &str = "menu:edit:undo";
pub const EVENT_EDIT_REDO: &str = "menu:edit:redo";
pub const EVENT_FILE_RECENT: &str = "menu:file:recent";
pub const EVENT_RECENT_CHANGED: &str = "menu:recent:changed";
pub const EVENT_APP_QUIT: &str = "menu:app:quit";
pub const EVENT_APP_QUIT_REQUESTED: &str = "menu:app:quit-requested";

pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let quit = MenuItemBuilder::with_id(APP_QUIT, "Выйти из WLite")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;

    let app_menu = build_app_submenu(app, &quit)?;

    let new_document = MenuItemBuilder::with_id(FILE_NEW, "Новый")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let open = MenuItemBuilder::with_id(FILE_OPEN, "Открыть…")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
    let recent = build_recent_submenu(app)?;
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
        .item(&recent)
        .separator()
        .item(&save)
        .item(&save_as)
        .separator()
        .item(&print)
        .separator()
        .item(&close_document)
        .item(&close_window)
        .build()?;

    let edit_menu = build_edit_submenu(app)?;

    let window_menu = SubmenuBuilder::new(app, "Окно")
        .minimize_with_text("Свернуть")
        .maximize_with_text("Масштаб")
        .build()?;

    MenuBuilder::new(app)
        .items(&[&app_menu, &file_menu, &edit_menu, &window_menu])
        .build()
}

fn build_edit_submenu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    let undo = MenuItemBuilder::with_id(EDIT_UNDO, "Отменить")
        .accelerator("CmdOrCtrl+Z")
        .build(app)?;
    let redo = MenuItemBuilder::with_id(EDIT_REDO, "Повторить")
        .accelerator("CmdOrCtrl+Shift+Z")
        .build(app)?;
    let cut = PredefinedMenuItem::cut(app, Some("Вырезать"))?;
    let copy = PredefinedMenuItem::copy(app, Some("Копировать"))?;
    let paste = PredefinedMenuItem::paste(app, Some("Вставить"))?;
    let select_all = PredefinedMenuItem::select_all(app, Some("Выделить все"))?;

    SubmenuBuilder::new(app, "Правка")
        .item(&undo)
        .item(&redo)
        .separator()
        .item(&cut)
        .item(&copy)
        .item(&paste)
        .separator()
        .item(&select_all)
        .build()
}

fn build_app_submenu<R: Runtime>(
    app: &AppHandle<R>,
    quit: &tauri::menu::MenuItem<R>,
) -> tauri::Result<tauri::menu::Submenu<R>> {
    let submenu = SubmenuBuilder::new(app, "WLite").about_with_text("О WLite", None);

    #[cfg(target_os = "macos")]
    let submenu = submenu
        .separator()
        .services_with_text("Службы")
        .separator()
        .hide_with_text("Скрыть WLite")
        .hide_others_with_text("Скрыть остальные")
        .show_all_with_text("Показать все");

    submenu.separator().item(quit).build()
}

fn build_recent_submenu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    let entries = recent_entries(app);
    let mut items: Vec<tauri::menu::MenuItem<R>> = Vec::new();

    if entries.is_empty() {
        items.push(
            MenuItemBuilder::with_id(RECENT_EMPTY, "Нет недавних документов")
                .enabled(false)
                .build(app)?,
        );
    } else {
        for (index, entry) in entries.iter().enumerate() {
            let label = recent_label(entry, &entries);
            items.push(
                MenuItemBuilder::with_id(format!("{RECENT_PREFIX}{index}"), label).build(app)?,
            );
        }
    }

    let clear = if entries.is_empty() {
        None
    } else {
        Some(MenuItemBuilder::with_id(RECENT_CLEAR, "Очистить список").build(app)?)
    };

    let mut builder = SubmenuBuilder::new(app, "Недавние");

    for item in &items {
        builder = builder.item(item);
    }

    if let Some(clear) = &clear {
        builder = builder.separator().item(clear);
    }

    builder.build()
}

fn recent_label(entry: &RecentEntry, entries: &[RecentEntry]) -> String {
    let duplicates = entries
        .iter()
        .filter(|candidate| candidate.file_name == entry.file_name)
        .count();

    if duplicates <= 1 {
        return entry.file_name.clone();
    }

    let parent = Path::new(&entry.path)
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    if parent.is_empty() {
        entry.file_name.clone()
    } else {
        format!("{} — {}", entry.file_name, parent)
    }
}

fn recent_store<R: Runtime>(app: &AppHandle<R>) -> Option<RecentStore> {
    app.path().app_data_dir().ok().map(RecentStore::new)
}

fn recent_entries<R: Runtime>(app: &AppHandle<R>) -> Vec<RecentEntry> {
    recent_store(app)
        .and_then(|store| store.list().ok())
        .unwrap_or_default()
}

pub fn refresh_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    app.set_menu(menu)?;
    Ok(())
}

fn clear_recent<R: Runtime>(app: &AppHandle<R>) {
    if let Some(store) = recent_store(app) {
        if let Err(error) = store.clear() {
            tracing::warn!(%error, "не удалось очистить недавние документы");
        }
    }

    if let Err(error) = refresh_menu(app) {
        tracing::warn!(%error, "не удалось обновить меню после очистки недавних");
    }

    if let Err(error) = app.emit(EVENT_RECENT_CHANGED, ()) {
        tracing::warn!(%error, event = EVENT_RECENT_CHANGED, "не удалось отправить событие");
    }
}

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let id: &str = event.id().as_ref();

    if id == FILE_PRINT {
        print_window(app);
        return;
    }

    if let Some(command) = id.strip_prefix(RECENT_PREFIX) {
        if command == "empty" {
            return;
        }

        if command == "clear" {
            clear_recent(app);
            return;
        }

        if let Ok(index) = command.parse::<usize>() {
            if let Some(entry) = recent_entries(app).get(index) {
                if let Err(error) = app.emit(EVENT_FILE_RECENT, entry.path.clone()) {
                    tracing::warn!(%error, event = EVENT_FILE_RECENT, "не удалось отправить событие");
                }
            }
        }

        return;
    }

    let event_name = match id {
        FILE_NEW => EVENT_FILE_NEW,
        FILE_OPEN => EVENT_FILE_OPEN,
        FILE_SAVE => EVENT_FILE_SAVE,
        FILE_SAVE_AS => EVENT_FILE_SAVE_AS,
        FILE_CLOSE_DOCUMENT => EVENT_FILE_CLOSE_DOCUMENT,
        FILE_CLOSE_WINDOW => EVENT_FILE_CLOSE_WINDOW,
        EDIT_UNDO => EVENT_EDIT_UNDO,
        EDIT_REDO => EVENT_EDIT_REDO,
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
