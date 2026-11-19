export type PdfPaper = "a4" | "letter";
export type PdfOrientation = "portrait" | "landscape";

export interface PdfMargins {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export interface PdfOptions {
  paper: PdfPaper;
  orientation: PdfOrientation;
  margins: PdfMargins;
  header: string;
  footer: string;
  pageNumbers: boolean;
}

export type MarginPresetId = "normal" | "narrow" | "wide" | "custom";

export interface MarginPreset {
  id: MarginPresetId;
  margin: number | null;
}

export const MARGIN_PRESETS: MarginPreset[] = [
  { id: "normal", margin: 20 },
  { id: "narrow", margin: 12 },
  { id: "wide", margin: 30 },
  { id: "custom", margin: null },
];

export const PDF_OPTIONS_CACHE_KEY = "liteoffice.docs.pdf.v1";

export const DEFAULT_PDF_OPTIONS: PdfOptions = {
  paper: "a4",
  orientation: "portrait",
  margins: { top: 20, right: 20, bottom: 20, left: 20 },
  header: "",
  footer: "",
  pageNumbers: true,
};

const PAPERS = new Set<PdfPaper>(["a4", "letter"]);
const ORIENTATIONS = new Set<PdfOrientation>(["portrait", "landscape"]);
const MAX_PAGE_TEXT = 200;
const MAX_MARGIN_MM = 100;
const MIN_CONTENT_MM = 20;

export function paperDimensionsMm(
  paper: PdfPaper,
  orientation: PdfOrientation,
): [number, number] {
  const [width, height] = paper === "letter" ? [215.9, 279.4] : [210, 297];

  return orientation === "landscape" ? [height, width] : [width, height];
}

export function marginPresetFor(margins: PdfMargins): MarginPresetId {
  const preset = MARGIN_PRESETS.find(
    (candidate) =>
      candidate.margin !== null &&
      candidate.margin === margins.top &&
      candidate.margin === margins.right &&
      candidate.margin === margins.bottom &&
      candidate.margin === margins.left,
  );

  return preset?.id ?? "custom";
}

export function marginsForPreset(preset: MarginPresetId): PdfMargins {
  const margin =
    MARGIN_PRESETS.find((candidate) => candidate.id === preset)?.margin ?? 20;

  return { top: margin, right: margin, bottom: margin, left: margin };
}

function normalizeMargins(value: unknown): PdfMargins {
  const record =
    value && typeof value === "object"
      ? (value as Record<string, unknown>)
      : {};

  const read = (key: keyof PdfMargins): number => {
    const candidate = record[key];
    return typeof candidate === "number" && Number.isFinite(candidate)
      ? candidate
      : DEFAULT_PDF_OPTIONS.margins[key];
  };

  return {
    top: read("top"),
    right: read("right"),
    bottom: read("bottom"),
    left: read("left"),
  };
}

export function normalizePdfOptions(value: unknown): PdfOptions {
  if (!value || typeof value !== "object") {
    return DEFAULT_PDF_OPTIONS;
  }

  const record = value as Record<string, unknown>;
  const paper =
    typeof record.paper === "string" && PAPERS.has(record.paper as PdfPaper)
      ? (record.paper as PdfPaper)
      : DEFAULT_PDF_OPTIONS.paper;
  const orientation =
    typeof record.orientation === "string" &&
    ORIENTATIONS.has(record.orientation as PdfOrientation)
      ? (record.orientation as PdfOrientation)
      : DEFAULT_PDF_OPTIONS.orientation;

  return {
    paper,
    orientation,
    margins: normalizeMargins(record.margins),
    header: typeof record.header === "string" ? record.header : "",
    footer: typeof record.footer === "string" ? record.footer : "",
    pageNumbers:
      typeof record.pageNumbers === "boolean"
        ? record.pageNumbers
        : DEFAULT_PDF_OPTIONS.pageNumbers,
  };
}

export function validatePdfOptions(options: PdfOptions): string | null {
  const margins: Array<[string, number]> = [
    ["Верхнее", options.margins.top],
    ["Правое", options.margins.right],
    ["Нижнее", options.margins.bottom],
    ["Левое", options.margins.left],
  ];

  for (const [name, value] of margins) {
    if (
      !Number.isFinite(value) ||
      value < 0 ||
      value > MAX_MARGIN_MM
    ) {
      return `${name} поле должно быть от 0 до ${MAX_MARGIN_MM} мм`;
    }
  }

  const [width, height] = paperDimensionsMm(options.paper, options.orientation);

  if (options.margins.left + options.margins.right > width - MIN_CONTENT_MM) {
    return "Боковые поля оставляют слишком мало места для текста";
  }

  if (options.margins.top + options.margins.bottom > height - MIN_CONTENT_MM) {
    return "Верхнее и нижнее поля оставляют слишком мало места для текста";
  }

  for (const [name, text] of [
    ["Верхний", options.header],
    ["Нижний", options.footer],
  ] as Array<[string, string]>) {
    if (text.length > MAX_PAGE_TEXT) {
      return `${name} колонтитул длиннее ${MAX_PAGE_TEXT} символов`;
    }

    if (hasControlCharacters(text)) {
      return `${name} колонтитул содержит недопустимые символы`;
    }
  }

  return null;
}

function hasControlCharacters(text: string): boolean {
  return Array.from(text).some((character) => {
    const code = character.codePointAt(0) ?? 0;
    return code < 32;
  });
}

export function readCachedPdfOptions(
  storage: Storage | undefined = typeof window !== "undefined"
    ? window.localStorage
    : undefined,
): PdfOptions {
  if (!storage) {
    return DEFAULT_PDF_OPTIONS;
  }

  try {
    const raw = storage.getItem(PDF_OPTIONS_CACHE_KEY);
    return raw ? normalizePdfOptions(JSON.parse(raw)) : DEFAULT_PDF_OPTIONS;
  } catch {
    return DEFAULT_PDF_OPTIONS;
  }
}

export function writeCachedPdfOptions(
  options: PdfOptions,
  storage: Storage | undefined = typeof window !== "undefined"
    ? window.localStorage
    : undefined,
): void {
  if (!storage) {
    return;
  }

  try {
    storage.setItem(PDF_OPTIONS_CACHE_KEY, JSON.stringify(options));
  } catch {
    return;
  }
}
