use tauri::webview::WebviewBuilder;
use tauri::window::WindowBuilder;
use tauri::{AppHandle, LogicalPosition, LogicalSize, WebviewUrl, Window, WindowEvent};

pub const SIDEBAR_W: f64 = 68.0;
pub const TABBAR_H: f64 = 40.0;

/// Main window: no own webview; the React shell is a full-size child, content webviews go on top of it.
pub fn create_main_window(app: &AppHandle) -> tauri::Result<()> {
    let window = WindowBuilder::new(app, "main")
        .title("NC Workspaces")
        .inner_size(1280.0, 800.0)
        .min_inner_size(800.0, 500.0)
        .build()?;
    let size = window.inner_size()?.to_logical::<f64>(window.scale_factor()?);
    window.add_child(
        WebviewBuilder::new("shell", WebviewUrl::App("index.html".into())).auto_resize(),
        LogicalPosition::new(0.0, 0.0),
        size,
    )?;
    let w = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }) {
            relayout(&w);
        }
    });
    Ok(())
}

fn content_rect(window: &Window) -> tauri::Result<(LogicalPosition<f64>, LogicalSize<f64>)> {
    let size = window.inner_size()?.to_logical::<f64>(window.scale_factor()?);
    Ok((
        LogicalPosition::new(SIDEBAR_W, TABBAR_H),
        LogicalSize::new((size.width - SIDEBAR_W).max(0.0), (size.height - TABBAR_H).max(0.0)),
    ))
}

/// Keeps every content webview (label `ws-…`) in the content rectangle.
pub fn relayout(window: &Window) {
    let Ok((pos, size)) = content_rect(window) else { return };
    for wv in window.webviews() {
        if wv.label().starts_with("ws-") {
            let _ = wv.set_position(pos);
            let _ = wv.set_size(size);
        }
    }
}
