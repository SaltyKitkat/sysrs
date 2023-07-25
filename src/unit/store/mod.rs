use std::collections::{hash_map::Entry, HashMap, HashSet};

use tap::Tap;

use super::{Unit, UnitDeps, UnitEntry, UnitStatus};

#[derive(Debug)]
pub struct UnitState {
    status: UnitStatus,
    requires: HashSet<UnitEntry>,
    required_by: HashSet<UnitEntry>,
}

impl UnitState {
    fn uninit() -> Self {
        Self {
            status: UnitStatus::Uninit,
            requires: HashSet::new(),
            required_by: HashSet::new(),
        }
    }
}

impl UnitState {
    // todo
}

#[derive(Debug)]
pub struct UnitStoreImpl {
    map: HashMap<UnitEntry, Box<dyn Unit>>, // info in unit files
    state_map: HashMap<UnitEntry, UnitState>, // runtime info
                                            // dep_graph: (),                            // depinfo currently not used
}

impl UnitStoreImpl {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            state_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, unit: Box<dyn Unit>) {
        let entry: UnitEntry = (unit.as_ref() as &dyn Unit).into();
        let unit = match self.map.entry(entry.clone()) {
            Entry::Occupied(o) => o.into_mut().tap_mut(|o| **o = unit),
            Entry::Vacant(v) => v.insert(unit),
        };
        let deps = unit.deps();
        for u in &deps.required_by {
            self.state_map
                .entry(u.to_owned())
                .or_insert_with(|| UnitState::uninit())
                .requires
                .insert(entry.clone());
        }
        let entry = self
            .state_map
            .entry(entry)
            .or_insert_with(|| UnitState::uninit());
        entry.requires.extend(deps.requires);
        entry.required_by.extend(deps.required_by);
    }

    fn get(&self, entry: &UnitEntry) -> Option<&dyn Unit> {
        self.map.get(entry).map(AsRef::as_ref)
    }

    fn get_status(&self, entry: &UnitEntry) -> UnitStatus {
        todo!()
    }

    fn clear(&mut self) {
        self.map.clear()
    }

    async fn start(&mut self, entry: &UnitEntry) {

        // start deps;
        // start unit;
        // report status;
    }
    async fn stop(&mut self, entry: &UnitEntry) {
        // stop units dep on this? no
        // stop unit;
    }
    async fn restart(&mut self, entry: &UnitEntry) {
        // if not running, start
        // else restart unit
    }
}

// when spawn a task:
// 1. resolve deps, and add them to work queue
// 2. wait for deps to start (?
// 3. run start
pub struct WorkQueueImpl {
    queue: Vec<()>, // should be a FIFO
}

impl WorkQueueImpl {
    fn new() -> Self {
        Self { queue: Vec::new() }
    }
    fn push(&mut self, _unit: &dyn Unit) {
        todo!()
    }
}
