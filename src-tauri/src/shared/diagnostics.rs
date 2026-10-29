use log::LevelFilter;
use std::backtrace::Backtrace;
use std::panic::{self, AssertUnwindSafe};
use std::thread;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Runtime};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

const LOG_FILE: &str = "nc-workspaces";
const MAX_LOG_BYTES: u128 = 5_000_000;
const KEPT_LOG_FILES: usize = 5;

pub fn log_plugin<R: Runtime>() -> TauriPlugin<R> {
    let level = if cfg!(debug_assertions) { LevelFilter::Debug } else { LevelFilter::Info };
    let mut builder = tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(Target::new(TargetKind::LogDir { file_name: Some(LOG_FILE.into()) }))
        .max_file_size(MAX_LOG_BYTES)
        .rotation_strategy(RotationStrategy::KeepSome(KEPT_LOG_FILES))
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .level(level);
    if cfg!(debug_assertions) {
        builder = builder.target(Target::new(TargetKind::Stdout));
    }
    builder.build()
}

pub fn install_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let thread = thread::current();
        log::error!(
            target: "panic",
            "thread '{}' panicked at {}: {}\n{}",
            thread.name().unwrap_or("<unnamed>"),
            info.location().map_or_else(|| "<unknown>".into(), ToString::to_string),
            panic_message(info.payload()),
            Backtrace::force_capture()
        );
        default_hook(info);
    }));
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<non-string panic payload>")
}

pub fn guard<T>(context: &str, work: impl FnOnce() -> T) -> Option<T> {
    match panic::catch_unwind(AssertUnwindSafe(work)) {
        Ok(value) => Some(value),
        Err(_) => {
            log::error!("{context} panicked; carrying on");
            None
        }
    }
}

pub fn spawn(name: &str, work: impl FnOnce() + Send + 'static) {
    let context = name.to_string();
    let spawned = thread::Builder::new().name(context.clone()).spawn(move || {
        guard(&context, work);
    });
    if let Err(error) = spawned {
        log::error!("could not start thread {name}: {error}");
    }
}

pub fn log_startup(app: &AppHandle) {
    log::info!(
        "NC Workspaces {} on {}/{}, tauri {}, webview {}",
        app.package_info().version,
        std::env::consts::OS,
        std::env::consts::ARCH,
        tauri::VERSION,
        tauri::webview_version().unwrap_or_else(|error| format!("unknown ({error})"))
    );
    #[cfg(target_os = "linux")]
    {
        let variable = |name: &str| std::env::var(name).unwrap_or_else(|_| "-".into());
        let webkit_overrides: Vec<String> = std::env::vars()
            .filter(|(name, _)| name.starts_with("WEBKIT_"))
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        log::info!(
            "session {}, GDK_BACKEND {}, AppImage {}, WebKit overrides [{}]",
            variable("XDG_SESSION_TYPE"),
            variable("GDK_BACKEND"),
            std::env::var_os("APPIMAGE").is_some(),
            webkit_overrides.join(", ")
        );
    }
}

#[cfg(target_os = "linux")]
pub fn forward_glib_logs() {
    glib::log_set_writer_func(|level, fields| {
        let field = |key: &str| fields.iter().find(|field| field.key() == key).and_then(|field| field.value_str());
        log::log!(
            target: "glib",
            glib_level(level),
            "[{}] {}",
            field("GLIB_DOMAIN").unwrap_or("-"),
            field("MESSAGE").unwrap_or_default()
        );
        glib::LogWriterOutput::Handled
    });
}

#[cfg(target_os = "linux")]
fn glib_level(level: glib::LogLevel) -> log::Level {
    match level {
        glib::LogLevel::Error | glib::LogLevel::Critical => log::Level::Error,
        glib::LogLevel::Warning => log::Level::Warn,
        glib::LogLevel::Message | glib::LogLevel::Info => log::Level::Info,
        glib::LogLevel::Debug => log::Level::Trace,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/shared/diagnostics.rs"]
mod tests;
