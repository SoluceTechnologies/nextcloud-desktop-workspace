use std::collections::HashSet;
use std::sync::Mutex;

#[derive(Default)]
pub struct Presentation(pub Mutex<Shown>);

#[derive(Default)]
pub struct Shown {
    target: Option<String>,
    starting: HashSet<String>,
}

impl Shown {
    pub fn show(&mut self, target: Option<&str>) -> Option<String> {
        self.target = target.map(Into::into);
        self.target.clone().filter(|label| !self.starting.contains(label))
    }

    #[cfg(any(target_os = "linux", test))]
    pub fn start(&mut self, label: &str) {
        self.starting.insert(label.into());
    }

    pub fn ready(&mut self, label: &str) -> bool {
        self.starting.remove(label) && self.target.as_deref() == Some(label)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/window/presentation.rs"]
mod tests;
