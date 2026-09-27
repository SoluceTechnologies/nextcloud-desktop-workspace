use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};
use uuid::Uuid;

const SERVICE: &str = "nc-workspaces";

#[derive(Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub login: String,
    pub password: String,
}

impl Credentials {
    pub fn authorization(&self) -> String {
        format!("Basic {}", BASE64.encode(format!("{}:{}", self.login, self.password)))
    }
}

fn cache() -> MutexGuard<'static, HashMap<Uuid, Credentials>> {
    static CACHE: LazyLock<Mutex<HashMap<Uuid, Credentials>>> = LazyLock::new(Default::default);
    CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn entry(workspace_id: Uuid) -> keyring::Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, &workspace_id.simple().to_string())
}

fn read(entry: &keyring::Entry) -> Option<Credentials> {
    serde_json::from_str(&entry.get_password().ok()?).ok()
}

pub fn load(workspace_id: Uuid) -> Option<Credentials> {
    if let Some(credentials) = cache().get(&workspace_id) {
        return Some(credentials.clone());
    }
    let credentials = read(&entry(workspace_id).ok()?)?;
    cache().insert(workspace_id, credentials.clone());
    Some(credentials)
}

pub fn save(workspace_id: Uuid, credentials: &Credentials) -> Result<(), String> {
    let json = serde_json::to_string(credentials).map_err(|error| error.to_string())?;
    entry(workspace_id)
        .and_then(|entry| entry.set_password(&json))
        .map_err(|error| format!("Could not store the app password in the system keychain ({error})"))?;
    cache().insert(workspace_id, credentials.clone());
    Ok(())
}

pub fn forget_cached(workspace_id: Uuid) -> Option<Credentials> {
    cache().remove(&workspace_id)
}

pub fn delete(workspace_id: Uuid) -> Option<Credentials> {
    let cached = forget_cached(workspace_id);
    let stored = entry(workspace_id).ok().and_then(|entry| {
        let credentials = read(&entry);
        let _ = entry.delete_credential();
        credentials
    });
    cached.or(stored)
}
