use std::collections::HashSet;
use std::sync::Mutex;

// A new tab is shown only once its web process asks to navigate. Painting a view whose web
// process is still starting makes WebKitGTK block the main thread for up to 500 ms
// (DrawingAreaProxyCoordinatedGraphics::forceUpdateIfNeeded waits for a reply the starting
// process cannot send yet), which froze the whole app on every new tab.
#[derive(Default)]
pub struct Presentation(pub Mutex<Shown>);

#[derive(Default)]
pub struct Shown {
    target: Option<String>,
    starting: HashSet<String>,
}

impl Shown {
    // Records the content webview that should be visible and returns it if it can be shown now.
    pub fn show(&mut self, target: Option<&str>) -> Option<String> {
        self.target = target.map(Into::into);
        self.target.clone().filter(|label| !self.starting.contains(label))
    }

    #[cfg(any(target_os = "linux", test))]
    pub fn start(&mut self, label: &str) {
        self.starting.insert(label.into());
    }

    // True when the webview just became ready and is the one that should be visible.
    pub fn ready(&mut self, label: &str) -> bool {
        self.starting.remove(label) && self.target.as_deref() == Some(label)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/window/presentation.rs"]
mod tests;
