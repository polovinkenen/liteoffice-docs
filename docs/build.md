# Сборка и проверки

Runbook для локальной сборки, CI, fuzz и бенчмарков. Пользовательские команды и
описание приложения — в [README](../README.md).

## Требования

- Node.js 22+ и npm (CI использует `actions/setup-node` с `node-version: 22`);
- Rust 1.90+ (`rust-version = "1.90"` в `Cargo.toml`), компоненты `rustfmt` и
  `clippy`;
- `cargo-deny`: `cargo install cargo-deny --locked`;
- macOS: Xcode Command Line Tools;
- Windows: WebView2 (в современных версиях Windows уже установлен);
- Linux: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`,
  `patchelf`; для сборки AppImage дополнительно `libfuse2`.

Tauri CLI ставится через `npm ci` (`@tauri-apps/cli` в devDependencies), отдельная
глобальная установка не нужна.

## Make-цели

| Команда | Что делает |
|---|---|
| `make install` | `npm ci`. |
| `make dev` | `npm run dev:desktop` → `tauri dev`. |
| `make build` | `npm run build:desktop` → `tauri build` с бандлами. |
| `make lint` | `npm run lint` (ESLint). |
| `make typecheck` | `npm run typecheck` (`tsc --noEmit`). |
| `make test` | `npm run test` (Vitest). |
| `make test-rust` | `cargo test --workspace`. |
| `make fmt` | `cargo fmt --all`. |
| `make fmt-check` | `cargo fmt --all -- --check`. |
| `make clippy` | `cargo clippy --workspace --all-targets -- -D warnings`. |
| `make deny` | `cargo deny check -W unmaintained`. |
| `make check` | lint, typecheck, test, test-rust, fmt-check, clippy, deny. |

Фронтенд отдельно: `npm run build` выполняет `tsc --noEmit && vite build`, но не
собирает десктопное приложение. `npm run dev` поднимает только Vite на
`http://localhost:1420`; файловые команды в браузере недоступны (`isTauri()`).

## Локальная сборка

```bash
npm ci
npm run dev:desktop        # разработка
npx tauri build --no-bundle # release-бинарник без установщика
npx tauri build            # установщики для текущей ОС
npx tauri build --bundles dmg
npx tauri build --target aarch64-apple-darwin --bundles dmg
```

Артефакты появляются в `target/release/bundle/`:

- macOS: `dmg/` (при сборке под arm64 —
  `target/aarch64-apple-darwin/release/bundle/dmg/LiteOffice Docs_0.1.0_aarch64.dmg`);
- Windows: `msi/` и `nsis/`;
- Linux: `deb/` и `appimage/`.

В CI дополнительно проверяются пути `src-tauri/target/.../bundle/...` — на случай,
если Tauri выберет каталог `src-tauri` как корень target.

## CI workflows

- `ci.yml` — зонтичный workflow: на `pull_request`, push в `master`/`main` и ручной
  запуск вызывает `checks.yml`; на push и ручной запуск дополнительно вызывает
  `bundle.yml`.
- `checks.yml` — два job'а:
  - `checks` (Ubuntu 22.04): системные зависимости, `npm ci`, `npm run lint`,
    `npm run typecheck`, `npm run test`, `cargo fmt --all -- --check`,
    `cargo clippy --workspace --all-targets -- -D warnings`,
    `cargo test --workspace`, `cargo-deny` с `-W unmaintained`;
  - `checks-cross-platform` (macOS и Windows): `npm ci`, `npm run lint`,
    `npm run typecheck`, `npm run test`, `cargo test --workspace` и
    `cargo clippy --workspace --all-targets -- -D warnings` без Linux-пакетов.
- `bundle.yml` — матрица macOS arm64, Windows x64, Ubuntu 22.04; собирает
  `.dmg`, `.msi`+NSIS, `.deb`+`.AppImage` и загружает артефакты
  `liteoffice-docs-macos-arm64`, `liteoffice-docs-windows-x64`, `liteoffice-docs-linux-x64` на 30 дней.
- `release.yml` — по тегу `v*` сверяет версии `package.json` и
  `src-tauri/tauri.conf.json`, запускает checks и bundle, затем прикрепляет
  установщики к GitHub Release.
- `fuzz.yml` — еженедельно по расписанию и вручную запускает все fuzz-таргеты по
  60 секунд с `rss_limit_mb=2048`, при падении выгружает `fuzz/artifacts/<target>`.

## Fuzz

Каркас `cargo-fuzz` лежит в `fuzz/` и исключён из workspace (`exclude = ["fuzz"]`),
чтобы не замедлять обычные проверки. Таргеты: `markdown_import`, `html_import`,
`rtf_import`, `txt_import`, `docx_import`.

```bash
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --locked
cargo +nightly fuzz run markdown_import -- -max_total_time=60 -rss_limit_mb=2048
cargo +nightly fuzz run html_import -- -max_total_time=60 -rss_limit_mb=2048
cargo +nightly fuzz run rtf_import -- -max_total_time=60 -rss_limit_mb=2048
cargo +nightly fuzz run txt_import -- -max_total_time=60 -rss_limit_mb=2048
cargo +nightly fuzz run docx_import -- -max_total_time=60 -rss_limit_mb=2048
```

Входные данные длиннее 1 МиБ пропускаются. Упавшие примеры складываются в
`fuzz/artifacts/<target>/`, корпус — в `fuzz/corpus/<target>/`; оба каталога в
`.gitignore`.

## Бенчмарк производительности

`crates/formats/tests/perf.rs` генерирует документ ~300 страниц (300 × 25 абзацев) на
русском языке, делает прогревочный импорт и три замера. Это единственный
`#[ignore]`-тест: он же проверяет, что 300-страничный RTF не упирается в лимит
токенов, и выполняется последовательно, поэтому глобальный пик памяти не смешивается
с параллельным тестом. Обычный `cargo test --workspace` его не запускает.

```bash
cargo test -p formats --test perf -- --ignored --nocapture
LITEOFFICE_DOCS_PERF_MAX_MS=3000 LITEOFFICE_DOCS_PERF_MAX_MB=800 \
  cargo test -p formats --test perf -- --ignored --nocapture
```

Пороги по умолчанию: 2000 мс медианы и 500 МБ пикового прироста памяти на каждый
сценарий. Сценарии: импорт TXT, Markdown, HTML, RTF, DOCX, разбор JSON модели и
`PmDoc::validate`.
Вебвью, ввод, IPC, экспорт и PDF не измеряются.

## Troubleshooting Linux

- Ошибка сборки `webkit2gtk` — установите `libwebkit2gtk-4.1-dev` (именно 4.1, не
  4.0) и повторите `npm ci`.
- Ошибка AppIndicator — `libappindicator3-dev`.
- Ошибка SVG/иконок — `librsvg2-dev`.
- Ошибка `patchelf` при сборке AppImage — установите `patchelf`.
- AppImage не собирается из-за FUSE — установите `libfuse2` (CI добавляет его в
  bundle-workflow).
- Если `tauri` не найден, проверьте, что выполнен `npm ci` и используется
  `npx tauri`, а не глобальный бинарник.

## Подпись и нотаризация

Сейчас подпись не настроена: в `tauri.conf.json` и workflows нет конфигурации
сертификатов, установщики выходят неподписанными. При включении подписи сертификаты и
пароли нужно хранить в секретах CI и импортировать в хранилище ОС: Keychain на macOS,
хранилище сертификатов Windows. В репозиторий, артефакты и образы сертификаты класть
нельзя. Нотаризация macOS также не настроена.
