# Документация WLite

Индекс документов:

- [architecture.md](architecture.md) — слои и крейты, открытие/сохранение,
  автосохранение, настройки, assets, PDF, меню, вкладки и границы доверия.
- [formats.md](formats.md) — матрица форматов, поддержка узлов и атрибутов,
  деградации, политика ссылок, лимиты, golden/fuzz/perf.
- [limitations.md](limitations.md) — известные ограничения текущей итерации.
- [build.md](build.md) — требования, make-цели, сборка и артефакты, CI, fuzz,
  бенчмарк, troubleshooting Linux, подпись.
- [acceptance.md](acceptance.md) — матрица форматов, автоматические проверки,
  CI на всех ОС, ручной чек-лист приёмки и открытые пункты.
- [adr/0001-docx-crate.md](adr/0001-docx-crate.md) — выбор `docx-rs` для DOCX.
- [adr/0002-pdf-engine.md](adr/0002-pdf-engine.md) — выбор Typst для экспорта PDF.

Пользовательское описание и быстрый старт — в [README](../README.md).
