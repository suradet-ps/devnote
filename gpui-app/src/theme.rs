//! Design tokens (`docs/DESIGN.md`, mirrored from `src/app.css`).
//!
//! Every colour used by the UI comes from [`Theme`] - matching the token
//! surface documented in `AGENTS.md` section 2, including the dark override
//! block from `src/app.css`.

use gpui::{Hsla, rgb, rgba};

#[derive(Clone, Copy)]
pub struct Theme {
    pub canvas: Hsla,
    pub surface_soft: Hsla,
    pub surface_card: Hsla,
    pub surface_cream_strong: Hsla,
    pub surface_dark: Hsla,
    pub surface_dark_elevated: Hsla,
    pub surface_dark_soft: Hsla,
    pub hairline: Hsla,
    pub hairline_soft: Hsla,

    pub primary: Hsla,
    pub primary_active: Hsla,
    /// Reserved token (disabled CTA state) - kept from `src/app.css`.
    #[allow(dead_code)]
    pub primary_disabled: Hsla,
    /// Focus-ring token - reserved until keyboard focus rings land in GPUI.
    #[allow(dead_code)]
    pub accent_teal: Hsla,
    #[allow(dead_code)]
    pub accent_amber: Hsla,

    pub ink: Hsla,
    pub body: Hsla,
    pub muted: Hsla,
    pub muted_soft: Hsla,
    pub on_primary: Hsla,
    pub on_dark: Hsla,
    pub on_dark_soft: Hsla,

    pub error: Hsla,
    pub success: Hsla,

    /// Editor selection background: coral at 15%.
    pub selection: Hsla,
    /// Active-line wash.
    pub active_line: Hsla,
    /// Search match wash.
    pub match_highlight: Hsla,
    /// Current search match wash.
    pub match_current: Hsla,
    /// Dialog backdrop.
    pub backdrop: Hsla,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            canvas: rgb(0xfcfbf7).into(),
            surface_soft: rgb(0xf8f5ef).into(),
            surface_card: rgb(0xf4f1ea).into(),
            surface_cream_strong: rgb(0xece7dd).into(),
            surface_dark: rgb(0x181715).into(),
            surface_dark_elevated: rgb(0x252320).into(),
            surface_dark_soft: rgb(0x1f1e1b).into(),
            hairline: rgb(0xe2dec9).into(),
            hairline_soft: rgb(0xe8e4d3).into(),

            primary: rgb(0xcc785c).into(),
            primary_active: rgb(0xa9583e).into(),
            primary_disabled: rgb(0xe2dec9).into(),
            accent_teal: rgb(0x5db8a6).into(),
            accent_amber: rgb(0xe8a55a).into(),

            ink: rgb(0x2c2b29).into(),
            body: rgb(0x454542).into(),
            muted: rgb(0x7a7873).into(),
            muted_soft: rgb(0x9a9893).into(),
            on_primary: rgb(0xffffff).into(),
            on_dark: rgb(0xfaf9f5).into(),
            on_dark_soft: rgb(0xa09d96).into(),

            error: rgb(0xc64545).into(),
            success: rgb(0x5db872).into(),

            selection: rgba(0xcc785c26).into(),
            active_line: rgba(0xcc785c10).into(),
            match_highlight: rgba(0xe8a55a40).into(),
            match_current: rgba(0xcc785c52).into(),
            backdrop: rgba(0x14141366).into(),
        }
    }

    pub fn dark() -> Self {
        let mut theme = Self::light();
        theme.canvas = theme.surface_dark;
        theme.surface_soft = theme.surface_dark_elevated;
        theme.surface_card = theme.surface_dark_soft;
        theme.surface_cream_strong = rgb(0x2a2825).into();
        theme.hairline = rgb(0x383532).into();
        theme.hairline_soft = rgb(0x2e2c29).into();

        theme.ink = theme.on_dark;
        theme.body = theme.on_dark_soft;
        theme.muted = rgb(0x78756f).into();
        theme.muted_soft = rgb(0x5c5954).into();

        theme.selection = rgba(0xcc785c40).into();
        theme.active_line = rgba(0xcc785c14).into();
        theme.match_highlight = rgba(0xe8a55a40).into();
        theme.match_current = rgba(0xcc785c66).into();
        theme
    }

    pub fn for_dark(dark: bool) -> Self {
        if dark { Self::dark() } else { Self::light() }
    }
}
