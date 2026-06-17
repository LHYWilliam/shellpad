use ratatui::style::{Color, Modifier, Style};

/// Central theme containing all named styles used across the application.
/// Each screen uses `theme.field_name` instead of hardcoded colors.
#[derive(Debug, Clone)]
pub struct Theme {
    // -- Panel / Surface colors --
    /// Default background for all screens
    pub background: Color,
    /// Panel / dialog background
    pub surface: Color,
    /// Default border for inactive widgets
    pub surface_border: Color,

    // -- Text colors --
    /// Main body text
    pub text_primary: Color,
    /// Less important text (status bar, hints)
    pub text_secondary: Color,
    /// Empty state, placeholder
    pub text_disabled: Color,
    /// Text color when on selected/highlighted background
    pub text_on_selected: Color,

    // -- Accent colors --
    /// Active focus, primary actions (replaces Color::Yellow)
    pub accent_primary: Color,
    /// Success states (replaces Color::Green)
    pub accent_success: Color,
    /// Error states (replaces Color::Red)
    pub accent_error: Color,
    /// Warning states (replaces Color::Yellow in non-focus contexts)
    pub accent_warning: Color,
    /// Informational (replaces Color::Cyan)
    pub accent_info: Color,

    // -- Selection backgrounds --
    /// Background for selected items (lists, Properties fields)
    pub selection_bg: Color,
    /// Background for inline editing fields
    pub editing_bg: Color,
}

impl Theme {
    /// Exact replica of current 8-color behavior.
    /// Use this during transition — zero visual change from existing code.
    pub const fn default_simple() -> Self {
        Self {
            background: Color::Reset,
            surface: Color::Reset,
            surface_border: Color::Cyan,
            text_primary: Color::White,
            text_secondary: Color::DarkGray,
            text_disabled: Color::DarkGray,
            text_on_selected: Color::Black,
            accent_primary: Color::Yellow,
            accent_success: Color::Green,
            accent_error: Color::Red,
            accent_warning: Color::Yellow,
            accent_info: Color::Cyan,
            selection_bg: Color::Cyan,
            editing_bg: Color::Green,
        }
    }

    /// Truecolor dark theme (Catppuccin Mocha-inspired palette).
    pub const fn default_dark() -> Self {
        Self {
            background: Color::Rgb(30, 30, 46),                // mantle
            surface: Color::Rgb(24, 24, 37),                   // base
            surface_border: Color::Rgb(69, 71, 90),            // surface0
            text_primary: Color::Rgb(205, 214, 244),           // text
            text_secondary: Color::Rgb(147, 153, 178),         // subtext1
            text_disabled: Color::Rgb(108, 112, 134),          // overlay1
            text_on_selected: Color::Rgb(17, 17, 27),          // crust
            accent_primary: Color::Rgb(137, 180, 250),         // blue
            accent_success: Color::Rgb(166, 227, 161),         // green
            accent_error: Color::Rgb(243, 139, 168),           // red
            accent_warning: Color::Rgb(249, 226, 175),         // yellow
            accent_info: Color::Rgb(137, 220, 235),            // sky
            selection_bg: Color::Rgb(70, 82, 125),             // muted indigo-blue
            editing_bg: Color::Rgb(60, 95, 75),                 // muted green
        }
    }

    /// Style for a selected/highlighted list item.
    pub fn selected_style(&self) -> Style {
        Style::default()
            .fg(self.text_primary)
            .bg(self.selection_bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for an inline editing field.
    pub fn editing_style(&self) -> Style {
        Style::default()
            .fg(self.text_primary)
            .bg(self.editing_bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for a normal (unselected) list item.
    pub fn normal_style(&self) -> Style {
        Style::default().fg(self.text_primary)
    }

    /// Style for status bar / dim hints.
    pub fn dim_style(&self) -> Style {
        Style::default()
            .fg(self.text_secondary)
            .add_modifier(Modifier::DIM)
    }
}
