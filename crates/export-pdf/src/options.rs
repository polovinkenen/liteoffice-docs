use serde::{Deserialize, Serialize};

use crate::PdfError;

const MAX_MARGIN_MM: f64 = 100.0;
const MIN_CONTENT_MM: f64 = 20.0;
const MAX_PAGE_TEXT_CHARS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PaperSize {
    #[default]
    A4,
    Letter,
}

impl PaperSize {
    pub fn dimensions_mm(self) -> (f64, f64) {
        match self {
            Self::A4 => (210.0, 297.0),
            Self::Letter => (215.9, 279.4),
        }
    }

    pub fn typst_name(self) -> &'static str {
        match self {
            Self::A4 => "a4",
            Self::Letter => "us-letter",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

impl Orientation {
    pub fn is_landscape(self) -> bool {
        matches!(self, Self::Landscape)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Margins {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            top: 20.0,
            right: 20.0,
            bottom: 20.0,
            left: 20.0,
        }
    }
}

impl Margins {
    pub fn uniform(value: f64) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfOptions {
    pub paper: PaperSize,
    pub orientation: Orientation,
    pub margins: Margins,
    #[serde(default)]
    pub header: Option<String>,
    #[serde(default)]
    pub footer: Option<String>,
    #[serde(default = "default_page_numbers")]
    pub page_numbers: bool,
}

fn default_page_numbers() -> bool {
    true
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            paper: PaperSize::A4,
            orientation: Orientation::Portrait,
            margins: Margins::default(),
            header: None,
            footer: None,
            page_numbers: true,
        }
    }
}

impl PdfOptions {
    pub fn validate(&self) -> Result<(), PdfError> {
        for (name, value) in [
            ("верхнее", self.margins.top),
            ("правое", self.margins.right),
            ("нижнее", self.margins.bottom),
            ("левое", self.margins.left),
        ] {
            if !value.is_finite() || !(0.0..=MAX_MARGIN_MM).contains(&value) {
                return Err(PdfError::InvalidOptions(format!(
                    "{name} поле должно быть от 0 до {MAX_MARGIN_MM} мм"
                )));
            }
        }

        let (width, height) = self.paper.dimensions_mm();
        let (width, height) = if self.orientation.is_landscape() {
            (height, width)
        } else {
            (width, height)
        };

        if self.margins.left + self.margins.right > width - MIN_CONTENT_MM {
            return Err(PdfError::InvalidOptions(
                "боковые поля оставляют слишком мало места для текста".to_owned(),
            ));
        }

        if self.margins.top + self.margins.bottom > height - MIN_CONTENT_MM {
            return Err(PdfError::InvalidOptions(
                "верхнее и нижнее поля оставляют слишком мало места для текста".to_owned(),
            ));
        }

        for (name, text) in [("верхний", &self.header), ("нижний", &self.footer)] {
            if let Some(text) = text {
                if text.chars().count() > MAX_PAGE_TEXT_CHARS {
                    return Err(PdfError::InvalidOptions(format!(
                        "{name} колонтитул длиннее {MAX_PAGE_TEXT_CHARS} символов"
                    )));
                }

                if text.chars().any(char::is_control) {
                    return Err(PdfError::InvalidOptions(format!(
                        "{name} колонтитул содержит недопустимые символы"
                    )));
                }
            }
        }

        Ok(())
    }
}
