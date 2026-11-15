# Wordlite

Кроссплатформенный (macOS, Linux, Windows) редактор документов — аналог Word для
повседневных задач. Ядро на Rust, интерфейс на React и TipTap, оболочка на Tauri 2.

## Статус

Проект на этапе каркаса (этап 1). Работают окно приложения, базовый WYSIWYG-редактор,
валидация модели документа и IPC-команды. Конвертеры форматов и экспорт PDF — в
разработке.

Реализовано сейчас:
- редактор TipTap: заголовки, bold/italic, списки, цитаты, блоки кода, undo/redo;
- счётчик слов и индикатор версии Rust-ядра;
- светлая и тёмная темы (следует системной);
- модель документа на базе ProseMirror JSON с валидацией и лимитами;
- каркас workspace: `app-core`, `docmodel`, `src-tauri`;
- CI: проверки на Ubuntu, сборка бандлов на трёх ОС.

Планируется:
- шрифты, размеры, цвета, выравнивание, отступы и интервалы;
- таблицы с объединением ячеек, изображения, гиперссылки, колонтитулы;
- поиск и замена, автосохранение и восстановление сессии;
- импорт и экспорт DOCX, RTF, Markdown, HTML, TXT;
- экспорт PDF с постраничной вёрсткой (A4/Letter, поля, ориентация);
- установщики `.dmg`, `.msi`, `.AppImage`/`.deb`.

## Стек

| Слой | Технологии |
|---|---|
| Оболочка | Tauri 2.12 |
| Интерфейс | React 19, TypeScript 5.9, Vite 8, TipTap 3.34 |
| Ядро | Rust 1.90+ (MSRV), `serde`, `thiserror`, `tracing` |
| Форматы (план) | `docx`, `rtf-parser`, `pulldown-cmark`, `comrak`, `typst`/`typst-pdf` |

## Требования

- Node.js 22+ и npm;
- Rust 1.90+ (`rustup update stable`);
- macOS: Xcode Command Line Tools;
- Windows: WebView2 (в современных версиях Windows уже установлен);
- Linux: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`.

## Быстрый старт

```bash
npm ci
npm run dev:desktop
```

Запуск только веб-интерфейса в браузере (без Rust-ядра):

```bash
npm run dev
```

## Команды

| Команда | Назначение |
|---|---|
| `npm run dev:desktop` | запуск приложения в режиме разработки |
| `npm run build:desktop` | сборка приложения и бандлов |
| `npm run dev` | Vite dev-сервер на `http://localhost:1420` |
| `npm run build` | проверка типов и сборка фронтенда |
| `npm run lint` | ESLint |
| `npm run typecheck` | TypeScript без эмита |
| `npm run test` | frontend-тесты (Vitest) |
| `make check` | eslint, tsc, vitest, rustfmt, clippy |
| `cargo test --workspace` | Rust-тесты |

## Структура

```
.
├── src/                    React + TypeScript
│   ├── editor/             редактор и панель инструментов
│   ├── lib/                модель документа и Tauri IPC
│   └── i18n/               строки интерфейса
├── src-tauri/              окно, команды, capabilities, конфигурация Tauri
├── crates/
│   ├── app-core/           ошибки приложения и сериализация для IPC
│   └── docmodel/           модель ProseMirror JSON и валидация
├── Cargo.toml              Rust workspace
└── .github/workflows/      CI для macOS, Linux и Windows
```

## Модель документа

Источник истины — ProseMirror JSON (схема v1, корневой узел `doc`). Rust-ядро
проверяет структуру: тип корня, текстовые узлы без детей, лимиты глубины (256) и числа
узлов (1 000 000). Атрибуты узлов и марок хранятся как JSON, поэтому схема расширяется
без изменения Rust-типов.

## Ограничения

- Пагинация в редакторе приблизительная: документ отображается непрерывным потоком в
  A4-рамке. Точная постраничная вёрстка — при экспорте PDF.
- Поддержка DOCX/RTF будет частичной: сложные конструкции Word пропускаются с
  предупреждением.
- ODT, импорт PDF, сноски, оглавление, рецензирование и совместное редактирование
  в текущий объём не входят.

## Лицензия

MIT.
