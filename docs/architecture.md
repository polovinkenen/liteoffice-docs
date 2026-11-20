# Архитектура LiteOffice Docs

Документ описывает слои приложения, потоки открытия и сохранения, автосохранение,
экспорт PDF, нативное меню и состояние вкладок. Пользовательские сценарии и команды
собраны в [README](../README.md); матрица форматов — в [formats.md](formats.md).

## Схема

```text
React + TipTap (src/)
    │  invoke / listen (@tauri-apps/api)
    ▼
src-tauri/ (commands.rs, commands/*, menu.rs, AppState)
    │
    ├── app-core      файлы, assets, автосохранения, настройки, недавние
    ├── formats       реестр codec_for(DocumentFormat)
    │     └── formats-core + formats-txt/markdown/html/rtf/docx
    ├── export-pdf    Typst World, шрифты, PdfOptions
    └── docmodel      PmDoc / PmNode / PmMark, validate()
```

## Слои и крейты

| Слой | Крейт или каталог | Зона ответственности |
|---|---|---|
| Модель | `crates/docmodel` | ProseMirror JSON (`PmDoc`, `PmNode`, `PmMark`), схема v1, `PmDoc::validate()`: корень `doc`, текстовые узлы без детей, глубина 256, не более 1 000 000 узлов. |
| Контракты форматов | `crates/formats-core` | `DocumentFormat`, `DocumentCodec`, `ImportedDocument`, `WrittenDocument`, `WrittenFile`, `FormatError`, `WarningSink`, `is_safe_link`, извлечение текста и признак форматирования. |
| Кодеки | `crates/formats-txt`, `formats-markdown`, `formats-html`, `formats-rtf`, `formats-docx` | Импорт и экспорт конкретных форматов, собственные лимиты разбора и предупреждения о деградациях. |
| Реестр | `crates/formats` | `codec_for(DocumentFormat) -> &'static dyn DocumentCodec`, повторное чтение после записи в тестах. |
| Прикладное ядро | `crates/app-core` | `AppError` с полями `{ code, message }`, чтение файлов с лимитом 50 МиБ, атомарная запись, `AssetStore`, `AutosaveStore`, `SettingsStore`, `RecentStore`. |
| PDF | `crates/export-pdf` | Конвертация модели в Typst-разметку, виртуальные файлы изображений, встроенные шрифты Liberation, `render()`. |
| Оболочка | `src-tauri` | Tauri-команды, фоновые задачи `spawn_blocking`, нативное меню, события, `AppState`, плагины. |
| Интерфейс | `src` | React 19, TipTap, вкладки, диалоги, автосохранение, настройки, поиск, панель вставки. |

Контракт кодека (`DocumentCodec`) задаёт `read(&str)` и `write(&PmDoc)`, а также
готовые обёртки `decode`, `read_bytes`, `write_bytes`. По умолчанию `decode` требует
UTF-8 и снимает BOM; RTF переопределяет декодирование по `\ansicpg`, DOCX
переопределяет `read_bytes`/`write_bytes`, потому что формат бинарный.

## Поток открытия

1. Фронтенд вызывает диалог `pickOpenPaths()` из `src/features/file/dialogs.ts` и
   получает список путей. Фильтр: `md`, `markdown`, `txt`, `html`, `htm`, `rtf`, `docx`.
2. Для каждого пути вызывается `openDocument(path)` → Tauri-команда `open_document`.
3. В `src-tauri/src/commands/document.rs` команда уходит в `run_blocking`
   (`spawn_blocking`), чтобы не блокировать IPC-поток.
4. `open_document_sync`: формат определяется по расширению (`DocumentFormat::from_path`),
   путь канонизируется, `read_bytes` читает не более `MAX_TEXT_FILE_BYTES` = 50 МиБ.
5. `codec.read_bytes(bytes)` вызывает `decode` и `read`. DOCX перед разбором проверяет
   zip-архив на число файлов, распакованный объём, размер записи и сжатие.
6. `PmDoc::validate()` проверяет структуру, глубину и число узлов.
7. Rust возвращает `{ path, format, document, warnings }`. Фронтенд создаёт вкладку
   (`tabFromOpened`), сохраняет предупреждения и передаёт JSON в TipTap.

Повторное открытие уже открытого пути не создаёт дубликат: `findTabByPath` возвращает
существующую вкладку. Пустая чистая вкладка при открытии заменяется документом.

## Открытие файлов из ОС

Ассоциации заданы в `bundle.fileAssociations` (`docx`, `rtf`, `txt`, `md`/`markdown`,
`html`/`htm`, `role: Editor`, `rank: Default`). Tauri переносит их в Info.plist на
macOS, в реестр NSIS/MSI на Windows и в `.desktop` на Linux. На Linux `Exec=... %F`
и `MimeType` приходят из собственного шаблона `src-tauri/linux/main.desktop`; на
Windows `src-tauri/windows/installer-hooks.nsh` дополнительно пишет
`OpenWithProgids`, потому что встроенный NSIS-бандлер в режиме currentUser не создаёт
ключи расширений.

1. Первый запуск: `setup` читает `std::env::args_os().skip(1)`, второй экземпляр:
   `tauri-plugin-single-instance` передаёт `args` и `cwd`, macOS: `RunEvent::Opened`
   с `file://`-URL. Все три источника идут в `open_files::collect_paths`: отбрасываются
   флаги, относительные пути резолвятся от `cwd`, проверяются существование файла и
   поддержанное расширение, повторы удаляются.
2. Пока фронтенд не забрал буфер (`take_opened_paths`), пути копятся в
   `PendingOpenFiles`. После первого drain событие `app:open-files` отправляется
   напрямую, поэтому один файл не открывается дважды.
3. `useOpenedFiles` в `App.tsx` подписывается на событие, затем забирает буфер и
   вызывает `workspace.openPaths` — тот же путь, что у диалога и drag&drop, с
   дедупликацией вкладок и баннером ошибок.

`PendingOpenFiles` — managed state в `lib.rs`; сам разбор и тесты — в
`src-tauri/src/open_files.rs`.

## Поток сохранения

1. `saveActive` выбирает путь: для нового документа или «Сохранить как…» показывается
   выбор формата (`SAVE_FORMATS`), затем `pickSavePath`.
2. Для TXT при наличии форматирования фронтенд требует подтверждение потери разметки.
3. Вызывается `saveDocument(path, editor.getJSON())` → команда `save_document`.
4. Rust десериализует `PmDoc`, разрешает assets (`asset:<id>` → `data:<mime>;base64,...`)
   и в фоновом потоке вызывает `save_document_sync`.
5. `save_document_sync` проверяет формат по расширению, валидирует модель, вызывает
   `codec.write_bytes` и пишет файл атомарно: временный файл в каталоге назначения,
   `flush`, затем `persist`/rename.
6. Ответ `{ path, format, warnings }` обновляет вкладку, добавляет путь в недавние,
   отменяет отложенное автосохранение и удаляет черновик.

## Автосохранение и восстановление

- Правка помечает вкладку изменённой (`markDirty`) и планирует запись через 1500 мс
  (`AUTOSAVE_DEBOUNCE_MS`). Таймер отдельный для каждой вкладки.
- Документ с путём записывается сразу в сам файл тем же `save_document`; после успешной
  записи вкладка становится чистой, а черновик удаляется.
- Новый документ без пути сохраняется как черновик в `<app-data>/autosave/<docId>.json`
  через `autosave_document`. Запись валидируется, ограничена 20 МиБ; `docId` — до 64
  символов `[A-Za-z0-9_-]`.
- При старте `listAutosaves` возвращает сводки. Если пользователь подтверждает
  восстановление, `restoreAutosave` читает записи и `restoredTabs` создаёт вкладки с
  `initialDocument`. При отказе вызывается `clearAutosaves`.
- При закрытии вкладки, окна или выходе изменённые вкладки требуют подтверждения.
  При подтверждении отменяются таймеры и удаляются черновики; при отмене состояние не
  меняется.
- Выключение автосохранения в настройках отменяет запланированные таймеры, но не
  удаляет уже существующие черновики.

## Настройки, недавние и assets

- `SettingsStore` хранит `<app-data>/settings.json`: `theme` (`system`/`light`/`dark`)
  и `autosaveEnabled`. Файл ограничен 64 КиБ, отсутствие файла даёт значения по
  умолчанию. Интерфейс дополнительно кэширует настройки в `localStorage`
  (`liteoffice.docs.settings.v1`) и применяет тему через `data-theme`.
- `RecentStore` хранит `<app-data>/recent.json` — до 15 записей, новые сверху.
  Путь канонизируется, дубликаты объединяются. После `add_recent`, `remove_recent` и
  `clear_recent` Rust перестраивает подменю «Недавние» и отправляет событие
  `menu:recent:changed`.
- `AssetStore` хранит изображения в `<app-data>/assets`. Поддерживаются `png`, `jpg`,
  `jpeg`, `gif`, `webp`, размер до 15 МиБ. В модели хранится `src = "asset:<id>"`,
  при показе расширение `AssetImage` загружает data URL, а перед сохранением и PDF
  `resolve_document` заменяет `asset:` на `data:`. Автоматическая очистка assets не
  выполняется.

## Экспорт PDF

1. Диалог `PdfExportDialog` собирает `PdfOptions`: бумага A4/Letter, ориентация, поля,
   верхний и нижний колонтитулы, нумерация. Параметры кэшируются в `localStorage`
   (`liteoffice.docs.pdf.v1`), а не в `settings.json`.
2. Фронтенд проверяет параметры (`validatePdfOptions`) и вызывает `export_pdf` с
   документом, параметрами, необязательным `outPath` и заголовком.
3. Rust валидирует модель, разрешает assets и выбирает цель: переданный путь или
   временный файл в `<app-cache>/pdf-preview`. Имя проверяется на расширение `.pdf`.
4. `export-pdf` конвертирует `PmDoc` в Typst-разметку (`convert.rs`): абзацы, заголовки,
   списки, task-списки, таблицы со span, изображения, разметку и параметры страницы.
5. `PdfWorld` реализует `typst::World`: главный виртуальный файл `/main.typ`, файлы
   изображений в памяти, встроенные шрифты, `today() = None` для детерминизма.
6. `typst::compile::<PagedDocument>` и `typst_pdf::pdf` дают байты PDF; число страниц и
   предупреждения возвращаются вместе с результатом.
7. Предпросмотр открывается системным просмотрщиком через `open_pdf_preview`; команда
   принимает только существующий файл внутри каталога предпросмотра. Старые
   предпросмотры обрезаются до пяти.

Шрифты: `crates/export-pdf/assets/fonts` содержит Liberation Serif/Sans/Mono
(Regular, Bold, Italic, BoldItalic). `resolve_family` отображает `Times New Roman`,
`Times`, `Georgia`, `serif` в Liberation Serif; `Arial`, `Helvetica`, `Verdana`,
`sans-serif` — в Liberation Sans; `Courier New`, `Courier`, `monospace` — в Liberation
Mono. Неизвестное семейство заменяется на Liberation Serif с предупреждением.

## Нативное меню и события

Меню строится в `src-tauri/src/menu.rs` при `setup`. Пункты отправляют события, которые
слушает фронтенд (`src/features/file/menuEvents.ts`):

- файл: `menu:file:new`, `open`, `save`, `save-as`, `export-pdf`, `close-document`,
  `close-window`, `print-fallback`;
- правка: `menu:edit:undo`, `menu:edit:redo`;
- недавние: `menu:file:recent` (путь в payload), `menu:recent:changed`;
- приложение: `menu:app:quit`, `menu:app:quit-requested`.

Печать обрабатывается в Rust: `window.print()`; при ошибке отправляется
`menu:file:print-fallback`, и фронтенд вызывает `window.print()`. «Недавние» меню
строится из `RecentStore`; пункт «Очистить список» очищает файл и обновляет меню.
`AppState` хранит `dirty` и `allow_exit`. При `ExitRequested` с несохранёнными
изменениями Rust предотвращает выход и отправляет `menu:app:quit-requested`; команда
`quit_app` выставляет `allow_exit` и завершает приложение. Плагины `single-instance`
(фокус существующего окна) и `window-state` (геометрия окна) подключены в `lib.rs`.

## Вкладки и состояние

`DocumentTab` (`src/features/tabs/tabUtils.ts`) содержит `id`, `path`, `format`,
`fileName`, `dirty`, `busy`, `error`, `warnings`, `initialDocument`, `untitledNumber`,
`showStart`. Каждая вкладка рендерит собственный `DocumentPane` с отдельным экземпляром
TipTap и своей историей undo/redo; неактивные панели остаются смонтированными и скрыты
атрибутом `hidden`. Состояние вкладок и активного редактора хранится в
`useDocumentWorkspace`; ссылки (`tabsRef`, `editorsRef`) защищают обработчики от
устаревших замыканий. Заголовок окна — имя активного файла с маркером несохранённых
изменений; признак `anyDirty` синхронизируется в Rust через `set_document_dirty`.
Стартовый экран показывается в пустой вкладке (`showStart`) и заменяется документом при
открытии или начале ввода.

## Границы доверия и лимиты

| Где | Что проверяется |
|---|---|
| Webview | CSP из `tauri.conf.json`, capabilities `src-tauri/capabilities/default.json`: только базовые команды окна, диалоги и `ask`. Команды вызываются через IPC. |
| IPC | Типизированные аргументы (`serde`), единый формат ошибок `{ code, message }`, `validate_document` и `set_document_dirty`. Ошибки модели и файлов возвращаются как `invalid_document`, `unsupported_format`, `file_too_large`, `io_error`, `invalid_argument`, `internal_error`. |
| Пути файлов | Rust не проверяет, что путь выбран пользователем в диалоге: граница доверия — сам webview. Проверяются расширение поддерживаемого формата, размер файла и допустимость пути предпросмотра PDF. |
| Модель | `docmodel`: тип корня, отсутствие текста у нетекстовых узлов, глубина 256, узлы 1 000 000. |
| Кодеки | Собственные лимиты глубины, узлов, токенов, групп, изображений и zip-архива; опасные ссылки и HTML-теги отбрасываются. Подробности — в [formats.md](formats.md). |
| Assets | MIME по расширению, размер 15 МиБ, идентификатор не длиннее 64 символов. |
| PDF | Поля 0–100 мм, минимум 20 мм на содержимое, колонтитулы до 200 символов без управляющих символов; цель должна иметь расширение `.pdf`; предпросмотр — только файл внутри кэша. |

Файлы больше 50 МиБ не читаются вообще (`app-core/src/files.rs`), поэтому память на
разбор Markdown и HTML сверху ограничена размером входа. Запись всегда атомарная, чтобы
не повредить исходный файл при ошибке.
