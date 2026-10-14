//! Persisted user settings (`{app_data}/devnote/settings.json`).
//!
//! Mirrors the Tauri version's `Settings` store, minus the keys that do not
//! apply to the GPUI port yet (word wrap, indent size).

use serde::{Deserialize, Serialize};

use crate::files;
use crate::i18n::Locale;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum LocaleChoice {
    #[default]
    System,
    En,
    Th,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeChoice,
    pub font_size: f32,
    pub show_line_numbers: bool,
    pub locale: LocaleChoice,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::System,
            font_size: 14.0,
            show_line_numbers: true,
            locale: LocaleChoice::System,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let mut settings: Settings = files::read_json("settings.json").unwrap_or_default();
        settings.font_size = settings.font_size.clamp(9.0, 32.0);
        settings
    }

    /// Persist to `settings.json`; callers surface failures as a toast.
    pub fn save(&self) -> Result<(), String> {
        files::write_json("settings.json", self)
    }

    pub fn locale(&self) -> Locale {
        match self.locale {
            LocaleChoice::En => Locale::En,
            LocaleChoice::Th => Locale::Th,
            LocaleChoice::System => Locale::detect(),
        }
    }
}
