use std::collections::{hash_map::Entry, HashMap, VecDeque};

use tap::Tap;
use tokio::{sync::mpsc::Receiver, task::JoinHandle};

use super::{Unit, UnitDeps, UnitEntry};

type Item = Box<dyn Unit + Send>;

#[derive(Debug)]
pub(crate) struct UnitStore {
    map: HashMap<UnitEntry, Item>, // info in unit files
}

pub(crate) enum Action {
    Update(Item),
    Remove,
    Start,
    Stop,
    Restart,
}

pub(crate) struct Message(UnitEntry, Action); // todo

impl UnitStore {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                todo!()
            }
        })
    }

    fn insert(&mut self, unit: Item) {
        let entry: UnitEntry = (unit.as_ref() as &dyn Unit).into();
        let unit = match self.map.entry(entry.clone()) {
            Entry::Occupied(o) => o.into_mut().tap_mut(|o| **o = unit),
            Entry::Vacant(v) => v.insert(unit),
        };
    }

    fn get(&self, entry: &UnitEntry) -> Option<&(dyn Unit + Send)> {
        self.map.get(entry).map(AsRef::as_ref)
    }

    fn clear(&mut self) {
        self.map.clear()
    }
}

pub struct DepMgr {
    map: HashMap<UnitEntry, UnitDeps>,
}

impl DepMgr {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, unit: &dyn Unit) {
        let entry: UnitEntry = unit.into();
        let UnitDeps {
            requires,
            required_by,
        } = unit.deps().clone();
        for unit in &required_by {
            self.map
                .entry(unit.clone())
                .or_default()
                .requires
                .push(entry.clone());
        }
        match self.map.entry(entry) {
            Entry::Occupied(o) => o.into_mut().requires.extend(requires),
            Entry::Vacant(v) => {
                v.insert(UnitDeps {
                    requires,
                    required_by,
                });
            }
        }
    }

    pub fn do_with_deps(
        &self,
        unit: UnitEntry,
        mut action: impl FnMut(&UnitEntry),
        mut condition: impl FnMut(&UnitEntry) -> bool,
    ) {
        if !condition(&unit) {
            return;
        }
        let mut stack = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(unit);
        while let Some(current) = queue.pop_front() {
            if let Some(unit) = self.map.get(&current) {
                queue.extend(unit.requires.iter().filter(|u| condition(u)).cloned());
            }
            if !stack.contains(&current) {
                stack.push(current);
            }
        }
        for unit in stack {
            action(&unit);
        }
    }
}
