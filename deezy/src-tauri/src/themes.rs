use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    #[serde(rename = "bg-darkest")]
    pub bg_darkest: String,
    #[serde(rename = "bg-dark")]
    pub bg_dark: String,
    #[serde(rename = "bg-surface")]
    pub bg_surface: String,
    #[serde(rename = "bg-elevated")]
    pub bg_elevated: String,
    #[serde(rename = "bg-hover")]
    pub bg_hover: String,
    pub accent: String,
    #[serde(rename = "accent-hover")]
    pub accent_hover: String,
    #[serde(rename = "accent-dim")]
    pub accent_dim: String,
    #[serde(rename = "text-primary")]
    pub text_primary: String,
    #[serde(rename = "text-secondary")]
    pub text_secondary: String,
    #[serde(rename = "text-tertiary")]
    pub text_tertiary: String,
    pub success: String,
    pub error: String,
    pub warning: String,
    pub border: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTheme {
    pub name: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub version: String,
    pub colors: ThemeColors,
}

impl CustomTheme {
    pub fn validate(&self) -> Result<(), String> {
        // Validate theme name
        if self.name.trim().is_empty() {
            return Err("Theme name cannot be empty".to_string());
        }

        // Validate version
        if self.version.trim().is_empty() {
            return Err("Theme version cannot be empty".to_string());
        }

        // Validate all color values are valid CSS colors
        let colors = vec![
            ("bg-darkest", &self.colors.bg_darkest),
            ("bg-dark", &self.colors.bg_dark),
            ("bg-surface", &self.colors.bg_surface),
            ("bg-elevated", &self.colors.bg_elevated),
            ("bg-hover", &self.colors.bg_hover),
            ("accent", &self.colors.accent),
            ("accent-hover", &self.colors.accent_hover),
            ("accent-dim", &self.colors.accent_dim),
            ("text-primary", &self.colors.text_primary),
            ("text-secondary", &self.colors.text_secondary),
            ("text-tertiary", &self.colors.text_tertiary),
            ("success", &self.colors.success),
            ("error", &self.colors.error),
            ("warning", &self.colors.warning),
            ("border", &self.colors.border),
        ];

        for (name, value) in colors {
            if value.trim().is_empty() {
                return Err(format!("Color '{}' cannot be empty", name));
            }
            // Basic validation - should start with # or rgb or rgba or hsl or hsla
            let v = value.trim().to_lowercase();
            if !v.starts_with('#') && !v.starts_with("rgb") && !v.starts_with("hsl") {
                return Err(format!("Color '{}' has invalid format: {}", name, value));
            }
        }

        Ok(())
    }
}

pub fn themes_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let themes_dir = dir.join("themes");
    std::fs::create_dir_all(&themes_dir).map_err(|e| e.to_string())?;
    Ok(themes_dir)
}

pub fn list_custom_themes(app: &tauri::AppHandle) -> Result<Vec<String>, String> {
    let dir = themes_dir(app)?;
    let mut themes = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext == "json" {
                    if let Some(stem) = entry.path().file_stem() {
                        themes.push(stem.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    themes.sort();
    Ok(themes)
}

fn sanitize_theme_name(name: &str) -> Result<String, String> {
    let sanitized = name.trim().replace(' ', "_").to_lowercase();
    if sanitized.is_empty() {
        return Err("Theme name cannot be empty".to_string());
    }
    if sanitized.contains('.') || sanitized.contains('/') || sanitized.contains('\\') || sanitized.contains("..") {
        return Err("Theme name contains invalid characters".to_string());
    }
    Ok(sanitized)
}

pub fn load_custom_theme(app: &tauri::AppHandle, theme_name: &str) -> Result<CustomTheme, String> {
    let dir = themes_dir(app)?;
    let filename = sanitize_theme_name(theme_name)?;
    let path = dir.join(format!("{}.json", filename));

    if !path.exists() {
        return Err(format!("Theme '{}' not found", theme_name));
    }

    let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let theme: CustomTheme = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    theme.validate()?;
    Ok(theme)
}

pub fn save_custom_theme(app: &tauri::AppHandle, theme: &CustomTheme) -> Result<(), String> {
    theme.validate()?;

    let dir = themes_dir(app)?;
    let filename = sanitize_theme_name(&theme.name)?;
    let path = dir.join(format!("{}.json", filename));

    let data = serde_json::to_string_pretty(theme).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())
}

pub fn delete_custom_theme(app: &tauri::AppHandle, theme_name: &str) -> Result<(), String> {
    let dir = themes_dir(app)?;
    let filename = sanitize_theme_name(theme_name)?;
    let path = dir.join(format!("{}.json", filename));

    if !path.exists() {
        return Err(format!("Theme '{}' not found", theme_name));
    }

    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

pub fn export_current_theme(
    theme_name: String,
    author: Option<String>,
    description: Option<String>,
    is_light: bool,
) -> CustomTheme {
    let colors = if is_light {
        ThemeColors {
            bg_darkest: "#f5f5f5".to_string(),
            bg_dark: "#ffffff".to_string(),
            bg_surface: "#fafafa".to_string(),
            bg_elevated: "#f0f0f0".to_string(),
            bg_hover: "#e8e8e8".to_string(),
            accent: "#a238ff".to_string(),
            accent_hover: "#8e2de6".to_string(),
            accent_dim: "rgba(162, 56, 255, 0.08)".to_string(),
            text_primary: "#1a1a1a".to_string(),
            text_secondary: "#666666".to_string(),
            text_tertiary: "#999999".to_string(),
            success: "#1db954".to_string(),
            error: "#e74c3c".to_string(),
            warning: "#f39c12".to_string(),
            border: "#e0e0e0".to_string(),
        }
    } else {
        ThemeColors {
            bg_darkest: "#0a0a0a".to_string(),
            bg_dark: "#121212".to_string(),
            bg_surface: "#1a1a1a".to_string(),
            bg_elevated: "#242424".to_string(),
            bg_hover: "#2a2a2a".to_string(),
            accent: "#a238ff".to_string(),
            accent_hover: "#b55bff".to_string(),
            accent_dim: "rgba(162, 56, 255, 0.15)".to_string(),
            text_primary: "#ffffff".to_string(),
            text_secondary: "#a0a0a0".to_string(),
            text_tertiary: "#666666".to_string(),
            success: "#1db954".to_string(),
            error: "#e74c3c".to_string(),
            warning: "#f39c12".to_string(),
            border: "#2a2a2a".to_string(),
        }
    };

    CustomTheme {
        name: theme_name,
        author,
        description,
        version: "1.0.0".to_string(),
        colors,
    }
}

pub fn create_example_themes(app: &tauri::AppHandle) -> Result<(), String> {
    // Midnight Blue theme
    let midnight_blue = CustomTheme {
        name: "Midnight Blue".to_string(),
        author: Some("Deezy Team".to_string()),
        description: Some("A deep blue theme inspired by the midnight sky".to_string()),
        version: "1.0.0".to_string(),
        colors: ThemeColors {
            bg_darkest: "#0a0e1a".to_string(),
            bg_dark: "#0f1729".to_string(),
            bg_surface: "#162038".to_string(),
            bg_elevated: "#1d2947".to_string(),
            bg_hover: "#243356".to_string(),
            accent: "#4a9eff".to_string(),
            accent_hover: "#6bb0ff".to_string(),
            accent_dim: "rgba(74, 158, 255, 0.15)".to_string(),
            text_primary: "#e8f0ff".to_string(),
            text_secondary: "#a0b8d9".to_string(),
            text_tertiary: "#6b7a94".to_string(),
            success: "#1db954".to_string(),
            error: "#ff5757".to_string(),
            warning: "#ffb347".to_string(),
            border: "#243356".to_string(),
        },
    };

    // Forest Green theme
    let forest_green = CustomTheme {
        name: "Forest Green".to_string(),
        author: Some("Deezy Team".to_string()),
        description: Some("A calming green theme inspired by nature".to_string()),
        version: "1.0.0".to_string(),
        colors: ThemeColors {
            bg_darkest: "#0a1410".to_string(),
            bg_dark: "#0f1f18".to_string(),
            bg_surface: "#162b21".to_string(),
            bg_elevated: "#1d372a".to_string(),
            bg_hover: "#244333".to_string(),
            accent: "#4ade80".to_string(),
            accent_hover: "#6ee7a0".to_string(),
            accent_dim: "rgba(74, 222, 128, 0.15)".to_string(),
            text_primary: "#e8fff2".to_string(),
            text_secondary: "#a0d9b8".to_string(),
            text_tertiary: "#6b9480".to_string(),
            success: "#22c55e".to_string(),
            error: "#ef4444".to_string(),
            warning: "#f59e0b".to_string(),
            border: "#244333".to_string(),
        },
    };

    save_custom_theme(app, &midnight_blue)?;
    save_custom_theme(app, &forest_green)?;

    // Sunset Orange theme
    let sunset_orange = CustomTheme {
        name: "Sunset Orange".to_string(),
        author: Some("Deezy Team".to_string()),
        description: Some("A warm orange theme inspired by beautiful sunsets".to_string()),
        version: "1.0.0".to_string(),
        colors: ThemeColors {
            bg_darkest: "#1a0e0a".to_string(),
            bg_dark: "#291510".to_string(),
            bg_surface: "#381c16".to_string(),
            bg_elevated: "#47231c".to_string(),
            bg_hover: "#562a22".to_string(),
            accent: "#ff7b3d".to_string(),
            accent_hover: "#ff9563".to_string(),
            accent_dim: "rgba(255, 123, 61, 0.15)".to_string(),
            text_primary: "#fff5f0".to_string(),
            text_secondary: "#d9b8a0".to_string(),
            text_tertiary: "#94806b".to_string(),
            success: "#1db954".to_string(),
            error: "#ff5757".to_string(),
            warning: "#ffb347".to_string(),
            border: "#562a22".to_string(),
        },
    };

    save_custom_theme(app, &sunset_orange)?;

    // Purple Haze theme
    let purple_haze = CustomTheme {
        name: "Purple Haze".to_string(),
        author: Some("Deezy Team".to_string()),
        description: Some("A vibrant purple theme with dreamy, mystical vibes".to_string()),
        version: "1.0.0".to_string(),
        colors: ThemeColors {
            bg_darkest: "#0f0a1a".to_string(),
            bg_dark: "#1a0f29".to_string(),
            bg_surface: "#251638".to_string(),
            bg_elevated: "#301d47".to_string(),
            bg_hover: "#3b2456".to_string(),
            accent: "#a855f7".to_string(),
            accent_hover: "#c084fc".to_string(),
            accent_dim: "rgba(168, 85, 247, 0.15)".to_string(),
            text_primary: "#f5f0ff".to_string(),
            text_secondary: "#c9b8d9".to_string(),
            text_tertiary: "#8b7a94".to_string(),
            success: "#1db954".to_string(),
            error: "#ff5757".to_string(),
            warning: "#ffb347".to_string(),
            border: "#3b2456".to_string(),
        },
    };

    // Ocean Teal theme
    let ocean_teal = CustomTheme {
        name: "Ocean Teal".to_string(),
        author: Some("Deezy Team".to_string()),
        description: Some("A refreshing teal theme inspired by tropical ocean waters".to_string()),
        version: "1.0.0".to_string(),
        colors: ThemeColors {
            bg_darkest: "#0a1a1a".to_string(),
            bg_dark: "#0f2929".to_string(),
            bg_surface: "#163838".to_string(),
            bg_elevated: "#1d4747".to_string(),
            bg_hover: "#245656".to_string(),
            accent: "#14b8a6".to_string(),
            accent_hover: "#2dd4bf".to_string(),
            accent_dim: "rgba(20, 184, 166, 0.15)".to_string(),
            text_primary: "#f0ffff".to_string(),
            text_secondary: "#b8d9d9".to_string(),
            text_tertiary: "#7a9494".to_string(),
            success: "#10b981".to_string(),
            error: "#ef4444".to_string(),
            warning: "#f59e0b".to_string(),
            border: "#245656".to_string(),
        },
    };

    // Crimson Red theme
    let crimson_red = CustomTheme {
        name: "Crimson Red".to_string(),
        author: Some("Deezy Team".to_string()),
        description: Some("A bold red theme with intense, passionate energy".to_string()),
        version: "1.0.0".to_string(),
        colors: ThemeColors {
            bg_darkest: "#1a0a0a".to_string(),
            bg_dark: "#290f0f".to_string(),
            bg_surface: "#381616".to_string(),
            bg_elevated: "#471d1d".to_string(),
            bg_hover: "#562424".to_string(),
            accent: "#ef4444".to_string(),
            accent_hover: "#f87171".to_string(),
            accent_dim: "rgba(239, 68, 68, 0.15)".to_string(),
            text_primary: "#fff0f0".to_string(),
            text_secondary: "#d9b8b8".to_string(),
            text_tertiary: "#947a7a".to_string(),
            success: "#1db954".to_string(),
            error: "#dc2626".to_string(),
            warning: "#ffb347".to_string(),
            border: "#562424".to_string(),
        },
    };

    // Golden Amber theme
    let golden_amber = CustomTheme {
        name: "Golden Amber".to_string(),
        author: Some("Deezy Team".to_string()),
        description: Some("A luxurious golden theme with warm, rich amber tones".to_string()),
        version: "1.0.0".to_string(),
        colors: ThemeColors {
            bg_darkest: "#1a140a".to_string(),
            bg_dark: "#291f0f".to_string(),
            bg_surface: "#382b16".to_string(),
            bg_elevated: "#47371d".to_string(),
            bg_hover: "#564324".to_string(),
            accent: "#f59e0b".to_string(),
            accent_hover: "#fbbf24".to_string(),
            accent_dim: "rgba(245, 158, 11, 0.15)".to_string(),
            text_primary: "#fffaf0".to_string(),
            text_secondary: "#d9c9b8".to_string(),
            text_tertiary: "#94887a".to_string(),
            success: "#1db954".to_string(),
            error: "#ff5757".to_string(),
            warning: "#fb923c".to_string(),
            border: "#564324".to_string(),
        },
    };

    // Rose Pink theme
    let rose_pink = CustomTheme {
        name: "Rose Pink".to_string(),
        author: Some("Deezy Team".to_string()),
        description: Some("A soft pink theme with elegant, romantic aesthetics".to_string()),
        version: "1.0.0".to_string(),
        colors: ThemeColors {
            bg_darkest: "#1a0a14".to_string(),
            bg_dark: "#290f1f".to_string(),
            bg_surface: "#38162b".to_string(),
            bg_elevated: "#471d37".to_string(),
            bg_hover: "#562443".to_string(),
            accent: "#ec4899".to_string(),
            accent_hover: "#f472b6".to_string(),
            accent_dim: "rgba(236, 72, 153, 0.15)".to_string(),
            text_primary: "#fff0f8".to_string(),
            text_secondary: "#d9b8cc".to_string(),
            text_tertiary: "#947a88".to_string(),
            success: "#1db954".to_string(),
            error: "#ff5757".to_string(),
            warning: "#ffb347".to_string(),
            border: "#562443".to_string(),
        },
    };

    // Slate Gray theme
    let slate_gray = CustomTheme {
        name: "Slate Gray".to_string(),
        author: Some("Deezy Team".to_string()),
        description: Some("A professional gray theme with modern, minimalist design".to_string()),
        version: "1.0.0".to_string(),
        colors: ThemeColors {
            bg_darkest: "#0f1419".to_string(),
            bg_dark: "#1a1f29".to_string(),
            bg_surface: "#242b38".to_string(),
            bg_elevated: "#2e3747".to_string(),
            bg_hover: "#384356".to_string(),
            accent: "#64748b".to_string(),
            accent_hover: "#94a3b8".to_string(),
            accent_dim: "rgba(100, 116, 139, 0.15)".to_string(),
            text_primary: "#f1f5f9".to_string(),
            text_secondary: "#cbd5e1".to_string(),
            text_tertiary: "#94a3b8".to_string(),
            success: "#1db954".to_string(),
            error: "#ff5757".to_string(),
            warning: "#ffb347".to_string(),
            border: "#384356".to_string(),
        },
    };

    // Cherry Blossom theme
    let cherry_blossom = CustomTheme {
        name: "Cherry Blossom".to_string(),
        author: Some("Deezy Team".to_string()),
        description: Some("A delicate theme inspired by Japanese cherry blossoms in spring".to_string()),
        version: "1.0.0".to_string(),
        colors: ThemeColors {
            bg_darkest: "#1a0f14".to_string(),
            bg_dark: "#291a1f".to_string(),
            bg_surface: "#38242b".to_string(),
            bg_elevated: "#472e37".to_string(),
            bg_hover: "#563843".to_string(),
            accent: "#fb7185".to_string(),
            accent_hover: "#fda4af".to_string(),
            accent_dim: "rgba(251, 113, 133, 0.15)".to_string(),
            text_primary: "#fff5f7".to_string(),
            text_secondary: "#fecdd3".to_string(),
            text_tertiary: "#9f8a8e".to_string(),
            success: "#1db954".to_string(),
            error: "#ff5757".to_string(),
            warning: "#ffb347".to_string(),
            border: "#563843".to_string(),
        },
    };

    // Cyber Neon theme
    let cyber_neon = CustomTheme {
        name: "Cyber Neon".to_string(),
        author: Some("Deezy Team".to_string()),
        description: Some("A futuristic cyberpunk theme with electric neon accents".to_string()),
        version: "1.0.0".to_string(),
        colors: ThemeColors {
            bg_darkest: "#0a0a1a".to_string(),
            bg_dark: "#0f0f29".to_string(),
            bg_surface: "#161638".to_string(),
            bg_elevated: "#1d1d47".to_string(),
            bg_hover: "#242456".to_string(),
            accent: "#00ffff".to_string(),
            accent_hover: "#5dffff".to_string(),
            accent_dim: "rgba(0, 255, 255, 0.15)".to_string(),
            text_primary: "#f0ffff".to_string(),
            text_secondary: "#b8e6e6".to_string(),
            text_tertiary: "#7a9999".to_string(),
            success: "#00ff88".to_string(),
            error: "#ff0055".to_string(),
            warning: "#ffaa00".to_string(),
            border: "#00ffff".to_string(),
        },
    };

    save_custom_theme(app, &purple_haze)?;
    save_custom_theme(app, &ocean_teal)?;
    save_custom_theme(app, &crimson_red)?;
    save_custom_theme(app, &golden_amber)?;
    save_custom_theme(app, &rose_pink)?;
    save_custom_theme(app, &slate_gray)?;
    save_custom_theme(app, &cherry_blossom)?;
    save_custom_theme(app, &cyber_neon)?;

    Ok(())
}
