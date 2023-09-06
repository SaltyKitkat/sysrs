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

pub(crate) struct Message(UnitEntry, Action);

impl UnitStore {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let entry = msg.0;
                match msg.1 {
                    Action::Update(unit) => self.insert(entry, unit),
                    Action::Remove => {
                        self.map.remove(&entry);
                    }
                    Action::Start => {
                        if let Some(unit) = self.get(&entry) {
                            todo!("start a unit")
                        } else {
                            todo!("handle missing unit")
                        }
                    }
                    Action::Stop => todo!(),
                    Action::Restart => todo!(),
                }
            }
        })
    }

    fn insert(&mut self, entry: UnitEntry, unit: Item) {
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

    // pub fn insert(&mut self, unit: &dyn Unit) {
    //     let entry: UnitEntry = unit.into();
    //     let UnitDeps {
    //         requires,
    //         required_by,
    //     } = unit.deps().clone();
    //     for unit in &required_by {
    //         self.map
    //             .entry(unit.clone())
    //             .or_default()
    //             .requires
    //             .push(entry.clone());
    //     }
    //     match self.map.entry(entry) {
    //         Entry::Occupied(o) => o.into_mut().requires.extend(requires),
    //         Entry::Vacant(v) => {
    //             v.insert(UnitDeps {
    //                 requires,
    //                 required_by,
    //             });
    //         }
    //     }
    // }

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
