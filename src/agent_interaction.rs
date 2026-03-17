use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::oneshot;
use once_cell::sync::Lazy;

pub struct InteractionManager {
    pending: Mutex<HashMap<String, oneshot::Sender<String>>>,
}

impl InteractionManager {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&self, prompt_id: String, tx: oneshot::Sender<String>) {
        self.pending.lock().unwrap().insert(prompt_id, tx);
    }

    pub fn resolve(&self, prompt_id: &str, answer: String) -> bool {
        if let Some(tx) = self.pending.lock().unwrap().remove(prompt_id) {
            let _ = tx.send(answer);
            true
        } else {
            false
        }
    }
}

pub static MANAGER: Lazy<Arc<InteractionManager>> = Lazy::new(|| Arc::new(InteractionManager::new()));
