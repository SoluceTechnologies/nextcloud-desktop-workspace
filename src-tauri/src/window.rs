use crate::model::Appearance;
use crate::runtime::{engine, EffectResult};
use crate::webviews;
use std::error::Error;
use std::sync::Mutex;
use tauri::webview::WebviewBuilder;
use tauri::window::{Color, WindowBuilder};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Theme, Webview, WebviewUrl, Window, WindowEvent};

pub const SIDEBAR_WIDTH: f64 = 68.0;
pub const TABBAR_HEIGHT: f64 = 40.0;

const DARK_CHROME: Color = Color(30, 31, 34, 255);
const LIGHT_CHROME: Color = Color(233, 234, 237, 255);

type Rect = (LogicalPosition<f64>, LogicalSize<f64>);

#[derive(Default)]
pub struct Fullscreen(pub Mutex<Option<String>>);

pub fn create_main_window(app: &AppHandle) -> tauri::Result<()> {
    let appearance = engine(app).state.theme;
    let builder = WindowBuilder::new(app, "main")
        .title("NC Workspaces")
        .theme(window_theme(appearance))
        .inner_size(1280.0, 800.0)
        .min_inner_size(800.0, 500.0);
    #[cfg(target_os = "macos")]
    let builder = builder.title_bar_style(tauri::TitleBarStyle::Transparent);
    #[cfg(target_os = "linux")]
    gtk_theme::remember_system_theme();
    let window = builder.build()?;
    window.add_child(
        WebviewBuilder::new("shell", WebviewUrl::App("index.html".into())).auto_resize(),
        LogicalPosition::new(0.0, 0.0),
        logical_size(&window)?,
    )?;
    if let Ok(theme) = window.theme() {
        paint_chrome(&window, theme);
    }
    let observed = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => relayout(&observed),
        WindowEvent::ThemeChanged(theme) => paint_chrome(&observed, *theme),
        _ => {}
    });
    Ok(())
}

pub fn main_window(app: &AppHandle) -> Result<Window, Box<dyn Error>> {
    app.get_window("main").ok_or_else(|| "main window missing".into())
}

pub fn apply_theme(app: &AppHandle, appearance: Appearance) -> EffectResult {
    let window = main_window(app)?;
    window.set_theme(window_theme(appearance))?;
    #[cfg(target_os = "linux")]
    app.run_on_main_thread(move || gtk_theme::apply(appearance))?;
    paint_chrome(&window, window.theme()?);
    Ok(())
}

// WebKitGTK derives prefers-color-scheme from the GTK theme name too, and tao only
// strips a "-dark" theme suffix at window creation, so switching to Light at
// runtime under e.g. Yaru-dark kept everything dark.
#[cfg(target_os = "linux")]
mod gtk_theme {
    use crate::model::Appearance;
    use gtk::prelude::GtkSettingsExt;
    use std::sync::OnceLock;

    const DARK_SUFFIXES: [&str; 5] = ["-dark", "-Dark", "-darker", "-Darker", ":dark"];

    static SYSTEM_THEME: OnceLock<Option<String>> = OnceLock::new();

    pub fn remember_system_theme() {
        SYSTEM_THEME.get_or_init(|| gtk::Settings::default()?.gtk_theme_name().map(Into::into));
    }

    pub fn apply(appearance: Appearance) {
        let Some(settings) = gtk::Settings::default() else { return };
        if let Some(system) = SYSTEM_THEME.get().cloned().flatten() {
            let name = match appearance {
                Appearance::Light => light_variant(&system),
                _ => &system,
            };
            settings.set_gtk_theme_name(Some(name));
        }
        settings.set_gtk_application_prefer_dark_theme(appearance == Appearance::Dark);
    }

    pub fn light_variant(name: &str) -> &str {
        DARK_SUFFIXES.iter().find_map(|suffix| name.strip_suffix(suffix)).unwrap_or(name)
    }
}

pub fn content_rect(window: &Window) -> tauri::Result<Rect> {
    let size = logical_size(window)?;
    Ok((
        LogicalPosition::new(SIDEBAR_WIDTH, TABBAR_HEIGHT),
        LogicalSize::new((size.width - SIDEBAR_WIDTH).max(0.0), (size.height - TABBAR_HEIGHT).max(0.0)),
    ))
}

pub fn relayout(window: &Window) {
    for webview in content_webviews(window) {
        if let Ok((position, size)) = rect_for(window, webview.label()) {
            let _ = webview.set_bounds(bounds(position, size));
        }
    }
}

pub fn show_only(app: &AppHandle, target_label: Option<&str>) -> EffectResult {
    let window = main_window(app)?;
    for webview in content_webviews(&window) {
        if Some(webview.label()) == target_label {
            let (position, size) = rect_for(&window, webview.label())?;
            webview.set_bounds(bounds(position, size))?;
            webview.show()?;
            webview.set_focus()?;
        } else {
            webview.hide()?;
        }
    }
    Ok(())
}

// A single set_bounds call, rather than separate set_position/set_size calls: on
// Linux/GTK, wry's webview.bounds() (used internally to fill in the field the other
// setter doesn't touch) only tracks position under X11 and otherwise reports (0, 0),
// so two separate calls made the second silently wipe out the first one's position.
fn bounds(position: LogicalPosition<f64>, size: LogicalSize<f64>) -> tauri::Rect {
    tauri::Rect { position: position.into(), size: size.into() }
}

fn content_webviews(window: &Window) -> Vec<Webview> {
    window.webviews().into_iter().filter(|webview| webviews::is_content(webview.label())).collect()
}

fn rect_for(window: &Window, label: &str) -> tauri::Result<Rect> {
    let fullscreen_label = window.app_handle().state::<Fullscreen>();
    let is_fullscreen = fullscreen_label.0.lock().is_ok_and(|current| current.as_deref() == Some(label));
    if is_fullscreen {
        return Ok((LogicalPosition::new(0.0, 0.0), logical_size(window)?));
    }
    content_rect(window)
}

fn logical_size(window: &Window) -> tauri::Result<LogicalSize<f64>> {
    Ok(window.inner_size()?.to_logical::<f64>(window.scale_factor()?))
}

fn window_theme(appearance: Appearance) -> Option<Theme> {
    match appearance {
        Appearance::System => None,
        Appearance::Light => Some(Theme::Light),
        Appearance::Dark => Some(Theme::Dark),
    }
}

fn paint_chrome(window: &Window, theme: Theme) {
    let color = if theme == Theme::Dark { DARK_CHROME } else { LIGHT_CHROME };
    let _ = window.set_background_color(Some(color));
}
