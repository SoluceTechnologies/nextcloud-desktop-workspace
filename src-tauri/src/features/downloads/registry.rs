use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub const KEPT: usize = 20;
pub const CANCELLED: &str = "cancelled";
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Active,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    pub id: Uuid,
    pub file_name: String,
    pub path: Option<String>,
    pub received: u64,
    pub total: Option<u64>,
    pub state: State,
    pub error: Option<String>,
    pub cancellable: bool,
}

#[derive(Default)]
pub struct Registry {
    downloads: Vec<Download>,
    last_reported: HashMap<Uuid, (Instant, u64)>,
}

impl Registry {
    pub fn start(&mut self, id: Uuid, file_name: String, path: Option<String>, cancellable: bool, now: Instant) -> Download {
        let download =
            Download { id, file_name, path, received: 0, total: None, state: State::Active, error: None, cancellable };
        self.downloads.push(download.clone());
        self.last_reported.insert(id, (now, 0));
        while self.downloads.len() > KEPT {
            let Some(oldest) = self.downloads.iter().position(|download| download.state != State::Active) else { break };
            self.downloads.remove(oldest);
        }
        download
    }

    pub fn progress(&mut self, id: Uuid, received: u64, total: Option<u64>, now: Instant) -> Option<Download> {
        let download = self.active_mut(id)?;
        download.received = received;
        download.total = total.filter(|&total| total > 0).or(download.total);
        let snapshot = download.clone();
        let (reported_at, reported_bytes) = self.last_reported.get(&id).copied().unwrap_or((now, 0));
        let first = reported_bytes == 0 && received > 0;
        let interval_passed = now.duration_since(reported_at) >= PROGRESS_INTERVAL;
        let percent_step = snapshot.total.is_some_and(|total| received.saturating_sub(reported_bytes) * 100 >= total);
        if !(first || interval_passed || percent_step) {
            return None;
        }
        self.last_reported.insert(id, (now, received));
        Some(snapshot)
    }

    pub fn finish(&mut self, id: Uuid, outcome: Result<(), String>) -> Option<Download> {
        let download = self.active_mut(id)?;
        match outcome {
            Ok(()) => download.state = State::Completed,
            Err(reason) if reason == CANCELLED => download.state = State::Cancelled,
            Err(reason) => (download.state, download.error) = (State::Failed, Some(reason)),
        }
        let snapshot = download.clone();
        self.last_reported.remove(&id);
        Some(snapshot)
    }

    pub fn list(&self) -> Vec<Download> {
        self.downloads.clone()
    }

    fn active_mut(&mut self, id: Uuid) -> Option<&mut Download> {
        self.downloads.iter_mut().find(|download| download.id == id && download.state == State::Active)
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/features/downloads/registry.rs"]
mod tests;
