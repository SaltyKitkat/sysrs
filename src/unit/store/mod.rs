use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

use super::{Unit, UnitEntry};

enum Req {
    Insert((oneshot::Sender<()>, Box<dyn Unit>)),
}
enum Resp {
    Insert(Result<(), Box<dyn Unit>>),
}

#[derive(Debug)]
pub struct UnitStoreImpl {
    map: HashMap<UnitEntry, Box<dyn Unit>>,
}

impl UnitStoreImpl {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, unit: Box<dyn Unit>) {
        let entry = (unit.as_ref() as &dyn Unit).into();
        self.map.insert(entry, unit);
    }

    fn get(&self, entry: &UnitEntry) -> Option<&dyn Unit> {
        self.map.get(entry).map(AsRef::as_ref)
    }

    fn clear(&mut self) {
        self.map.clear()
    }
}

// when spawn a task:
// 1. resolve deps, and add them to work queue
// 2. wait for deps to start (?
// 3. run start
pub struct WorkQueueImpl {
    queue: Vec<()>,
}

impl WorkQueueImpl {
    fn new() -> Self {
        Self { queue: Vec::new() }
    }
    fn push(&mut self, unit: &dyn Unit) {}
}
