pub mod channel;
pub mod db;
pub mod gemini;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Key: (channel_id, user_id)
pub type PendingQueries = Arc<Mutex<HashMap<(u64, u64), PendingQuery>>>;

pub struct PendingQuery {
    pub items: Vec<crate::tarkov::models::Item>,
    pub created_at: Instant,
}

pub fn new_pending_queries() -> PendingQueries {
    Arc::new(Mutex::new(HashMap::new()))
}
