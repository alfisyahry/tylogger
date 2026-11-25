/// Semantic theme system for Tylogger according to docs/DESIGN.md (Section 5).
/// Provides complete typed sRGB color tokens for Light, Dark, and Soft Dark palettes,
/// with lossless conversion to Win32 GDI COLORREF (0x00BBGGRR).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// Creates a Color from 8-bit RGB components.
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Creates a Color from 24-bit sRGB hex literal (e.g. `0xF4F5F7`).
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }

    /// Converts sRGB to Win32 GDI `COLORREF` (`0x00BBGGRR`).
    pub const fn to_colorref(self) -> u32 {
        ((self.b as u32) << 16) | ((self.g as u32) << 8) | (self.r as u32)
    }

    /// Computes the relative luminance according to WCAG 2.1 specifications.
    pub fn relative_luminance(&self) -> f64 {
        fn channel_lum(val: u8) -> f64 {
            let s = val as f64 / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel_lum(self.r) + 0.7152 * channel_lum(self.g) + 0.0722 * channel_lum(self.b)
    }

    /// Computes contrast ratio against another color (returns value in range 1.0..21.0).
    pub fn contrast_ratio(&self, other: &Color) -> f64 {
        let l1 = self.relative_luminance();
        let l2 = other.relative_luminance();
        let lighter = l1.max(l2);
        let darker = l1.min(l2);
        (lighter + 0.05) / (darker + 0.05)
    }
}

/// Pair of foreground and background colors for semantic severity tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeverityColors {
    pub fg: Color,
    pub bg: Color,
}

/// Complete semantic tokens according to docs/DESIGN.md Section 5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeTokens {
    pub window_bg: Color,
    pub surface_primary: Color,
    pub surface_secondary: Color,
    pub surface_hover: Color,
    pub surface_pressed: Color,
    pub border_subtle: Color,
    pub border_control: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_disabled: Color,
    pub accent_primary: Color,
    pub accent_on_solid: Color,
    pub focus_ring: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub selection_inactive: Color,
    pub row_bg: Color,
    pub row_alternate: Color,
    pub row_gutter: Color,
    pub search_match_bg: Color,
    pub search_match_fg: Color,
    pub search_current_bg: Color,
    pub search_current_fg: Color,

    // Severity tokens (Section 5.2)
    pub severity_trace: SeverityColors,
    pub severity_info: SeverityColors,
    pub severity_notice: SeverityColors,
    pub severity_warning: SeverityColors,
    pub severity_error: SeverityColors,
    pub severity_critical: SeverityColors,
}

/// Normative Light theme palette from docs/DESIGN.md Section 5.
pub static LIGHT_THEME: ThemeTokens = ThemeTokens {
    window_bg: Color::from_hex(0xF4F5F7),
    surface_primary: Color::from_hex(0xFFFFFF),
    surface_secondary: Color::from_hex(0xF7F8FA),
    surface_hover: Color::from_hex(0xEDF0F4),
    surface_pressed: Color::from_hex(0xE1E6ED),
    border_subtle: Color::from_hex(0xDFE3E9),
    border_control: Color::from_hex(0x929DAA),
    text_primary: Color::from_hex(0x232932),
    text_secondary: Color::from_hex(0x505D6D),
    text_muted: Color::from_hex(0x657181),
    text_disabled: Color::from_hex(0x929BA6),
    accent_primary: Color::from_hex(0x315F9A),
    accent_on_solid: Color::from_hex(0xFFFFFF),
    focus_ring: Color::from_hex(0x315F9A),
    selection_bg: Color::from_hex(0xDCE9FA),
    selection_fg: Color::from_hex(0x172C45),
    selection_inactive: Color::from_hex(0xE5E9EE),
    row_bg: Color::from_hex(0xFFFFFF),
    row_alternate: Color::from_hex(0xF8F9FB),
    row_gutter: Color::from_hex(0xF3F5F8),
    search_match_bg: Color::from_hex(0xF9E5A8),
    search_match_fg: Color::from_hex(0x422E08),
    search_current_bg: Color::from_hex(0xF1CB69),
    search_current_fg: Color::from_hex(0x302200),

    severity_trace: SeverityColors {
        fg: Color::from_hex(0x526780),
        bg: Color::from_hex(0xEDF2F8),
    },
    severity_info: SeverityColors {
        fg: Color::from_hex(0x286344),
        bg: Color::from_hex(0xE9F3ED),
    },
    severity_notice: SeverityColors {
        fg: Color::from_hex(0x315F9A),
        bg: Color::from_hex(0xE8F0FC),
    },
    severity_warning: SeverityColors {
        fg: Color::from_hex(0x81530A),
        bg: Color::from_hex(0xFCF2D8),
    },
    severity_error: SeverityColors {
        fg: Color::from_hex(0xA12D35),
        bg: Color::from_hex(0xFBECEE),
    },
    severity_critical: SeverityColors {
        fg: Color::from_hex(0x8E1830),
        bg: Color::from_hex(0xF9DFE5),
    },
};

/// Normative Dark theme palette from docs/DESIGN.md Section 5.
pub static DARK_THEME: ThemeTokens = ThemeTokens {
    window_bg: Color::from_hex(0x171A1F),
    surface_primary: Color::from_hex(0x1E2228),
    surface_secondary: Color::from_hex(0x252A32),
    surface_hover: Color::from_hex(0x2D343E),
    surface_pressed: Color::from_hex(0x363F4B),
    border_subtle: Color::from_hex(0x353C46),
    border_control: Color::from_hex(0x778392),
    text_primary: Color::from_hex(0xE5E8EC),
    text_secondary: Color::from_hex(0xB0BBC8),
    text_muted: Color::from_hex(0x99A5B3),
    text_disabled: Color::from_hex(0x737D89),
    accent_primary: Color::from_hex(0x91B8ED),
    accent_on_solid: Color::from_hex(0x182333),
    focus_ring: Color::from_hex(0x91B8ED),
    selection_bg: Color::from_hex(0x304763),
    selection_fg: Color::from_hex(0xEAF2FF),
    selection_inactive: Color::from_hex(0x30363F),
    row_bg: Color::from_hex(0x1E2228),
    row_alternate: Color::from_hex(0x22272E),
    row_gutter: Color::from_hex(0x191D23),
    search_match_bg: Color::from_hex(0x5B4925),
    search_match_fg: Color::from_hex(0xFFF0C2),
    search_current_bg: Color::from_hex(0x806126),
    search_current_fg: Color::from_hex(0xFFFFFF),

    severity_trace: SeverityColors {
        fg: Color::from_hex(0xA7BBD4),
        bg: Color::from_hex(0x273344),
    },
    severity_info: SeverityColors {
        fg: Color::from_hex(0x9DCCA9),
        bg: Color::from_hex(0x26382E),
    },
    severity_notice: SeverityColors {
        fg: Color::from_hex(0xA7C8F3),
        bg: Color::from_hex(0x28384E),
    },
    severity_warning: SeverityColors {
        fg: Color::from_hex(0xE5C183),
        bg: Color::from_hex(0x453923),
    },
    severity_error: SeverityColors {
        fg: Color::from_hex(0xF0A5A9),
        bg: Color::from_hex(0x462D33),
    },
    severity_critical: SeverityColors {
        fg: Color::from_hex(0xFFB1C1),
        bg: Color::from_hex(0x522A39),
    },
};

/// Normative Soft Dark theme palette from docs/DESIGN.md Section 5.
pub static SOFT_DARK_THEME: ThemeTokens = ThemeTokens {
    window_bg: Color::from_hex(0x20242A),
    surface_primary: Color::from_hex(0x282D34),
    surface_secondary: Color::from_hex(0x2D333B),
    surface_hover: Color::from_hex(0x363D46),
    surface_pressed: Color::from_hex(0x414A55),
    border_subtle: Color::from_hex(0x3E4651),
    border_control: Color::from_hex(0x84909E),
    text_primary: Color::from_hex(0xDEE3E9),
    text_secondary: Color::from_hex(0xB4BEC9),
    text_muted: Color::from_hex(0xA3AFBD),
    text_disabled: Color::from_hex(0x7F8995),
    accent_primary: Color::from_hex(0xA0BDE5),
    accent_on_solid: Color::from_hex(0x182333),
    focus_ring: Color::from_hex(0xA0BDE5),
    selection_bg: Color::from_hex(0x3A4C62),
    selection_fg: Color::from_hex(0xEFF5FF),
    selection_inactive: Color::from_hex(0x3C434D),
    row_bg: Color::from_hex(0x282D34),
    row_alternate: Color::from_hex(0x2B3038),
    row_gutter: Color::from_hex(0x242930),
    search_match_bg: Color::from_hex(0x5D4C2C),
    search_match_fg: Color::from_hex(0xFFF0C2),
    search_current_bg: Color::from_hex(0x82612E),
    search_current_fg: Color::from_hex(0xFFFFFF),

    severity_trace: SeverityColors {
        fg: Color::from_hex(0xAFBFD2),
        bg: Color::from_hex(0x303B49),
    },
    severity_info: SeverityColors {
        fg: Color::from_hex(0xA7CDB2),
        bg: Color::from_hex(0x303F35),
    },
    severity_notice: SeverityColors {
        fg: Color::from_hex(0xB2C9EA),
        bg: Color::from_hex(0x344257),
    },
    severity_warning: SeverityColors {
        fg: Color::from_hex(0xE1C28B),
        bg: Color::from_hex(0x493E2D),
    },
    severity_error: SeverityColors {
        fg: Color::from_hex(0xE9ABAE),
        bg: Color::from_hex(0x49363A),
    },
    severity_critical: SeverityColors {
        fg: Color::from_hex(0xF4B1BD),
        bg: Color::from_hex(0x543442),
    },
};

/// Supported theme modes in Tylogger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
    SoftDark,
    System,
    HighContrast,
}

impl ThemeMode {
    /// Returns the static theme tokens corresponding to this theme mode.
    pub fn tokens(&self) -> &'static ThemeTokens {
        match self {
            Self::Light | Self::System => &LIGHT_THEME,
            Self::Dark => &DARK_THEME,
            Self::SoftDark => &SOFT_DARK_THEME,
            Self::HighContrast => &LIGHT_THEME, // Fallback baseline
        }
    }

    /// Cycles through the primary interactive themes (Light -> Dark -> Soft Dark -> Light).
    pub fn cycle(&self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::SoftDark,
            Self::SoftDark | Self::System | Self::HighContrast => Self::Light,
        }
    }

    /// User-friendly display label.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::SoftDark => "Soft Dark",
            Self::System => "System",
            Self::HighContrast => "High Contrast",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_conversions_and_colorref() {
        // Red: #FF0000 -> COLORREF 0x000000FF
        let red = Color::from_rgb(255, 0, 0);
        assert_eq!(red.to_colorref(), 0x000000FF);

        // Green: #00FF00 -> COLORREF 0x0000FF00
        let green = Color::from_rgb(0, 255, 0);
        assert_eq!(green.to_colorref(), 0x0000FF00);

        // Blue: #0000FF -> COLORREF 0x00FF0000
        let blue = Color::from_rgb(0, 0, 255);
        assert_eq!(blue.to_colorref(), 0x00FF0000);

        // Window bg Light: #F4F5F7
        let light_bg = Color::from_hex(0xF4F5F7);
        assert_eq!(light_bg.r, 0xF4);
        assert_eq!(light_bg.g, 0xF5);
        assert_eq!(light_bg.b, 0xF7);
        assert_eq!(light_bg.to_colorref(), 0x00F7F5F4);
    }

    #[test]
    fn test_theme_tokens_exact_matches() {
        assert_eq!(LIGHT_THEME.window_bg, Color::from_hex(0xF4F5F7));
        assert_eq!(LIGHT_THEME.surface_primary, Color::from_hex(0xFFFFFF));
        assert_eq!(LIGHT_THEME.accent_primary, Color::from_hex(0x315F9A));

        assert_eq!(DARK_THEME.window_bg, Color::from_hex(0x171A1F));
        assert_eq!(DARK_THEME.surface_primary, Color::from_hex(0x1E2228));
        assert_eq!(DARK_THEME.accent_primary, Color::from_hex(0x91B8ED));

        assert_eq!(SOFT_DARK_THEME.window_bg, Color::from_hex(0x20242A));
        assert_eq!(SOFT_DARK_THEME.surface_primary, Color::from_hex(0x282D34));
        assert_eq!(SOFT_DARK_THEME.accent_primary, Color::from_hex(0xA0BDE5));
    }

    #[test]
    fn test_text_contrast_compliance() {
        // WCAG AA requirement: normal text contrast ratio >= 4.5:1
        let light_contrast = LIGHT_THEME
            .text_primary
            .contrast_ratio(&LIGHT_THEME.surface_primary);
        assert!(
            light_contrast >= 4.5,
            "Light theme text contrast {} must be >= 4.5",
            light_contrast
        );

        let dark_contrast = DARK_THEME
            .text_primary
            .contrast_ratio(&DARK_THEME.surface_primary);
        assert!(
            dark_contrast >= 4.5,
            "Dark theme text contrast {} must be >= 4.5",
            dark_contrast
        );

        let soft_dark_contrast = SOFT_DARK_THEME
            .text_primary
            .contrast_ratio(&SOFT_DARK_THEME.surface_primary);
        assert!(
            soft_dark_contrast >= 4.5,
            "Soft Dark theme text contrast {} must be >= 4.5",
            soft_dark_contrast
        );
    }

    #[test]
    fn test_theme_cycle() {
        let mut t = ThemeMode::Light;
        t = t.cycle();
        assert_eq!(t, ThemeMode::Dark);
        t = t.cycle();
        assert_eq!(t, ThemeMode::SoftDark);
        t = t.cycle();
        assert_eq!(t, ThemeMode::Light);
    }
}
