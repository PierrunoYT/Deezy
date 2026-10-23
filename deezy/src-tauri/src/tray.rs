use tauri::{
    AppHandle, Manager, Emitter,
    tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
    menu::{Menu, MenuItem, PredefinedMenuItem},
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tray menu texts. English until the frontend sends the translations for the
/// current language (`set_tray_labels`).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayLabels {
    pub show_hide: String,
    pub idle: String,
    pub active: String,
    pub pause: String,
    pub resume: String,
    pub quit: String,
}

impl Default for TrayLabels {
    fn default() -> Self {
        Self {
            show_hide: "Show/Hide".to_string(),
            idle: "No active downloads".to_string(),
            active: "Downloads in progress…".to_string(),
            pause: "Pause Downloads".to_string(),
            resume: "Resume Downloads".to_string(),
            quit: "Quit".to_string(),
        }
    }
}

impl TrayLabels {
    /// Keeps menu texts short; anything blank falls back to English.
    pub fn sanitized(self) -> Self {
        let defaults = Self::default();
        let clean = |value: String, fallback: String| {
            let value: String = value.trim().chars().take(60).collect();
            if value.is_empty() { fallback } else { value }
        };
        Self {
            show_hide: clean(self.show_hide, defaults.show_hide),
            idle: clean(self.idle, defaults.idle),
            active: clean(self.active, defaults.active),
            pause: clean(self.pause, defaults.pause),
            resume: clean(self.resume, defaults.resume),
            quit: clean(self.quit, defaults.quit),
        }
    }
}

#[derive(Clone)]
pub struct TrayState {
    pub downloads_active: Arc<Mutex<bool>>,
    pub downloads_paused: Arc<Mutex<bool>>,
    pub labels: Arc<Mutex<TrayLabels>>,
}

impl TrayState {
    pub fn new() -> Self {
        Self {
            downloads_active: Arc::new(Mutex::new(false)),
            downloads_paused: Arc::new(Mutex::new(false)),
            labels: Arc::new(Mutex::new(TrayLabels::default())),
        }
    }
}

pub fn create_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let labels = TrayLabels::default();
    let show_hide = MenuItem::with_id(app, "show_hide", &labels.show_hide, true, None::<&str>)?;
    let downloads_status = MenuItem::with_id(app, "downloads_status", &labels.idle, false, None::<&str>)?;
    let separator1 = PredefinedMenuItem::separator(app)?;
    let pause_resume = MenuItem::with_id(app, "pause_resume", &labels.pause, true, None::<&str>)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", &labels.quit, true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show_hide,
            &downloads_status,
            &separator1,
            &pause_resume,
            &separator2,
            &quit,
        ],
    )?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(get_tray_icon(app)?)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button, button_state, .. } = event {
                if button == MouseButton::Left && button_state == MouseButtonState::Up {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
            }
        })
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "show_hide" => {
                    if let Some(window) = app.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
                "pause_resume" => {
                    let _ = app.emit("tray-pause-resume", ());
                }
                "quit" => {
                    let _ = app.emit("app-exit-requested", ());
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

fn get_tray_icon(app: &AppHandle) -> Result<tauri::image::Image<'static>, Box<dyn std::error::Error>> {
    // Try to load the icon from the icons directory
    let icon_path = app.path().resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?
        .join("icons")
        .join("32x32.png");

    if icon_path.exists() {
        let icon_data = std::fs::read(&icon_path)
            .map_err(|e| format!("Failed to read icon file: {}", e))?;
        let img = image::load_from_memory(&icon_data)?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Ok(tauri::image::Image::new_owned(rgba.into_raw(), width, height))
    } else {
        // Fallback: use the app icon
        let icon_bytes = include_bytes!("../icons/32x32.png");
        let img = image::load_from_memory(icon_bytes)?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Ok(tauri::image::Image::new_owned(rgba.into_raw(), width, height))
    }
}

pub fn update_tray_menu(
    app: &AppHandle,
    downloads_active: bool,
    downloads_paused: bool,
    labels: &TrayLabels,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(tray) = app.tray_by_id("main") {
        // Recreate menu with updated state
        let show_hide = MenuItem::with_id(app, "show_hide", &labels.show_hide, true, None::<&str>)?;
        
        let status_text = if downloads_active {
            &labels.active
        } else {
            &labels.idle
        };
        let downloads_status = MenuItem::with_id(app, "downloads_status", status_text, false, None::<&str>)?;
        
        let separator1 = PredefinedMenuItem::separator(app)?;
        
        let pause_text = if downloads_paused {
            &labels.resume
        } else {
            &labels.pause
        };
        let pause_resume = MenuItem::with_id(
            app,
            "pause_resume",
            pause_text,
            downloads_active || downloads_paused,
            None::<&str>,
        )?;
        
        let separator2 = PredefinedMenuItem::separator(app)?;
        let quit = MenuItem::with_id(app, "quit", &labels.quit, true, None::<&str>)?;

        let menu = Menu::with_items(
            app,
            &[
                &show_hide,
                &downloads_status,
                &separator1,
                &pause_resume,
                &separator2,
                &quit,
            ],
        )?;

        tray.set_menu(Some(menu))?;

        // Update icon based on download status
        if downloads_active {
            let _ = tray.set_icon(Some(get_tray_icon_active(app)?));
        } else {
            let _ = tray.set_icon(Some(get_tray_icon(app)?));
        }
    }

    Ok(())
}

fn get_tray_icon_active(app: &AppHandle) -> Result<tauri::image::Image<'static>, Box<dyn std::error::Error>> {
    // For now, use the same icon. In the future, you could create a different icon
    // with a badge or different color to indicate active downloads
    get_tray_icon(app)
}

pub fn set_tray_tooltip(app: &AppHandle, tooltip: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_tooltip(Some(tooltip))?;
    }
    Ok(())
}
