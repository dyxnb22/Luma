//! Semantic color tokens and ASCII-safe symbols for the launcher TUI.

use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    /// Prefer light when `COLORFGBG` background looks bright.
    Auto,
}

/// Visual tokens for a compact Raycast-style launcher.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub accent: Color,
    pub muted: Color,
    pub text: Color,
    pub border: Color,
    pub border_focused: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub destructive: Color,
    pub permission: Color,
    pub overlay_dim: Color,
    /// Filled background for centered overlay panels (must not use terminal default Clear).
    pub panel_bg: Color,
    /// Full-screen wash behind quit confirm (light — covers the dark hub).
    pub quit_backdrop_bg: Color,
    /// Quit confirm panel — slightly distinct from [`quit_backdrop_bg`].
    pub quit_panel_bg: Color,
    pub quit_panel_fg: Color,
}

impl Theme {
    pub fn resolve(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
            ThemeMode::Auto => Self::detect(),
        }
    }

    /// Default dark terminal palette — restrained, not neon.
    pub fn dark() -> Self {
        Self {
            accent: Color::Cyan,
            muted: Color::DarkGray,
            text: Color::Gray,
            border: Color::DarkGray,
            border_focused: Color::Cyan,
            selected_bg: Color::Indexed(236),
            selected_fg: Color::White,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            destructive: Color::LightRed,
            permission: Color::Magenta,
            overlay_dim: Color::Indexed(234),
            panel_bg: Color::Indexed(236),
            // Soft light wash + slightly brighter panel (still readable distinction).
            quit_backdrop_bg: Color::Indexed(252),
            quit_panel_bg: Color::Indexed(255),
            quit_panel_fg: Color::Black,
        }
    }

    /// Light terminal palette for bright backgrounds.
    pub fn light() -> Self {
        Self {
            accent: Color::Blue,
            muted: Color::Gray,
            text: Color::Black,
            border: Color::Gray,
            border_focused: Color::Blue,
            selected_bg: Color::Indexed(251),
            selected_fg: Color::Black,
            success: Color::Green,
            warning: Color::Rgb(180, 110, 0),
            error: Color::Red,
            destructive: Color::Red,
            permission: Color::Magenta,
            overlay_dim: Color::Indexed(254),
            panel_bg: Color::Indexed(255),
            quit_backdrop_bg: Color::Indexed(254),
            quit_panel_bg: Color::Indexed(255),
            quit_panel_fg: Color::Black,
        }
    }

    pub fn detect() -> Self {
        if let Ok(cfg) = std::env::var("COLORFGBG") {
            if let Some(bg) = cfg.split(';').nth(1) {
                if let Ok(n) = bg.parse::<u16>() {
                    // Common convention: bg >= 7 means light-ish palette.
                    if n >= 7 {
                        return Self::light();
                    }
                }
            }
        }
        Self::dark()
    }

    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border(&self, focused: bool) -> Style {
        Style::default().fg(if focused {
            self.border_focused
        } else {
            self.border
        })
    }

    pub fn text(&self) -> Style {
        Style::default().fg(self.text)
    }

    pub fn muted(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn accent(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn selected_row(&self) -> Style {
        Style::default()
            .fg(self.selected_fg)
            .bg(self.selected_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn row(&self) -> Style {
        Style::default().fg(self.text)
    }

    pub fn module_badge(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn action_hint(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn success(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn error(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn destructive(&self) -> Style {
        Style::default()
            .fg(self.destructive)
            .add_modifier(Modifier::BOLD)
    }

    pub fn permission(&self) -> Style {
        Style::default().fg(self.permission)
    }

    pub fn key_hint(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn match_highlight(&self, selected: bool) -> Style {
        if selected {
            Style::default()
                .fg(self.accent)
                .bg(self.selected_bg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default()
                .fg(self.accent)
                .add_modifier(Modifier::BOLD)
        }
    }

    pub fn kind_style(&self, kind: ResultKindVisual, selected: bool) -> Style {
        let fg = match kind {
            ResultKindVisual::Normal => self.text,
            ResultKindVisual::Warming => self.muted,
            ResultKindVisual::Permission => self.permission,
            ResultKindVisual::Unavailable => self.warning,
            ResultKindVisual::NotConfigured => self.warning,
        };
        if selected {
            Style::default()
                .fg(if kind == ResultKindVisual::Normal {
                    self.selected_fg
                } else {
                    fg
                })
                .bg(self.selected_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(fg)
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Decorative glyphs with a pure-ASCII fallback (no Nerd Font required).
#[derive(Clone, Copy, Debug)]
pub struct Symbols {
    pub selected: &'static str,
    pub enter: &'static str,
    pub cursor: &'static str,
    pub up: &'static str,
    pub down: &'static str,
    pub ellipsis: &'static str,
    pub sep: &'static str,
}

impl Symbols {
    pub fn unicode() -> Self {
        Self {
            selected: "›",
            enter: "↵",
            cursor: "▌",
            up: "↑",
            down: "↓",
            ellipsis: "…",
            sep: "·",
        }
    }

    pub fn ascii() -> Self {
        Self {
            selected: ">",
            enter: "Ret",
            cursor: "|",
            up: "^",
            down: "v",
            ellipsis: "...",
            sep: "|",
        }
    }

    /// `LUMA_TUI_ASCII=1` forces ASCII; otherwise Unicode (module glyphs stay ASCII either way).
    pub fn detect() -> Self {
        match std::env::var("LUMA_TUI_ASCII") {
            Ok(v) if matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES") => Self::ascii(),
            _ => Self::unicode(),
        }
    }
}

/// Visual treatment derived from `SearchItem.kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultKindVisual {
    Normal,
    Warming,
    Permission,
    Unavailable,
    NotConfigured,
}

impl ResultKindVisual {
    pub fn from_kind(kind: &str) -> Self {
        match kind {
            "warming" => Self::Warming,
            "permission" | "permission_required" => Self::Permission,
            "unavailable" => Self::Unavailable,
            // Empty-store onboarding is actionable — do not badge as "not configured".
            "not_configured" | "not-configured" => Self::NotConfigured,
            _ => Self::Normal,
        }
    }

    pub fn badge(self) -> Option<&'static str> {
        match self {
            Self::Normal => None,
            Self::Warming => Some("loading"),
            Self::Permission => Some("permission"),
            Self::Unavailable => Some("unavailable"),
            Self::NotConfigured => Some("setup"),
        }
    }
}

fn module_key(module_id: &str) -> &str {
    module_id
        .strip_prefix("luma.")
        .unwrap_or(module_id)
        .split('.')
        .next()
        .unwrap_or(module_id)
}

fn humanize_key(key: &str) -> String {
    key.split(['-', '_'])
        .filter(|p| !p.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// ASCII-safe module glyph derived from module id (no hardcoded module table).
pub fn module_glyph(module_id: &str) -> String {
    let key = module_key(module_id);
    key.chars()
        .next()
        .map(|c| c.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| ".".into())
}

/// Friendly module label: prefer registry display_name, else derive from id.
pub fn module_label(
    module_id: &str,
    catalog: &std::collections::HashMap<String, String>,
) -> String {
    catalog
        .get(module_id)
        .cloned()
        .unwrap_or_else(|| humanize_key(module_key(module_id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn module_labels_prefer_catalog_then_derive() {
        let mut catalog = HashMap::new();
        catalog.insert("luma.media".into(), "Records".into());
        assert_eq!(module_label("luma.media", &catalog), "Records");
        assert_eq!(module_label("luma.apps", &catalog), "Apps");
        assert_eq!(module_glyph("luma.secrets"), "S");
        assert_eq!(module_label("luma.secrets", &catalog), "Secrets");
        assert_eq!(
            module_label("luma.custom-module", &catalog),
            "Custom Module"
        );
    }

    #[test]
    fn light_and_dark_differ() {
        assert_ne!(Theme::dark().accent, Theme::light().accent);
    }
}
