use std::path::Path;
use std::sync::OnceLock;
use tauri::Webview;
use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};

// Ubuntu's and Debian's WebKitGTK refuses every buffer transport when the display runs
// on NVIDIA's driver (their disable-nvidia-dmabuf patch, for
// https://bugs.webkit.org/show_bug.cgi?id=262607), so web views get no
// AcceleratedBackingStore. Any page that still enters accelerated compositing then
// crashes the whole app (AcceleratedBackingStore::update on null): View Transitions,
// and ordinary layouts that reach WebKit's unchecked compositing paths, as clicking
// Download in Nextcloud Files did. The shared-memory transport avoids the NVIDIA
// DMA-BUF bug the patch guards against and gives every web view its backing store.
// Policy Never keeps pages rendering exactly as they did without one.
const DRM_CLASS: &str = "/sys/class/drm";
const SHARED_MEMORY_RENDERER: [(&str, &str); 2] =
    [("WEBKIT_FORCE_DMABUF_RENDERER", "1"), ("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1")];
const USER_OVERRIDES: [&str; 4] = [
    "WEBKIT_FORCE_DMABUF_RENDERER",
    "WEBKIT_DMABUF_RENDERER_FORCE_SHM",
    "WEBKIT_DISABLE_DMABUF_RENDERER",
    "WEBKIT_DISABLE_COMPOSITING_MODE",
];

static SHARED_MEMORY: OnceLock<bool> = OnceLock::new();

// Must run before GTK and WebKit start, and before any other thread exists.
pub fn prepare() {
    let enabled = needs_shared_memory(nvidia_drives_display(Path::new(DRM_CLASS)), |name| {
        std::env::var_os(name).is_some()
    });
    if enabled {
        for (name, value) in SHARED_MEMORY_RENDERER {
            std::env::set_var(name, value);
        }
    }
    let _ = SHARED_MEMORY.set(enabled);
}

pub fn configure(webview: &Webview) {
    if !SHARED_MEMORY.get().copied().unwrap_or(false) {
        return;
    }
    let _ = webview.with_webview(|platform_webview| {
        if let Some(settings) = WebViewExt::settings(&platform_webview.inner()) {
            settings.set_hardware_acceleration_policy(HardwareAccelerationPolicy::Never);
        }
    });
}

fn needs_shared_memory(nvidia_display: bool, is_set: impl Fn(&str) -> bool) -> bool {
    nvidia_display && !USER_OVERRIDES.iter().any(|name| is_set(name))
}

// WebKit's main device is the GPU behind the display, which the compositor picks
// from the firmware's boot VGA device.
fn nvidia_drives_display(drm: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(drm) else { return false };
    entries.flatten().any(|entry| {
        let device = entry.path().join("device");
        let boot_vga = std::fs::read_to_string(device.join("boot_vga")).is_ok_and(|value| value.trim() == "1");
        let driver = std::fs::read_link(device.join("driver"));
        boot_vga && driver.is_ok_and(|driver| driver.file_name().is_some_and(|name| name == "nvidia"))
    })
}

#[cfg(test)]
#[path = "../../tests/unit/shared/rendering.rs"]
mod tests;
