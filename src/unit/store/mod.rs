use std::collections::{hash_map::Entry, HashMap, VecDeque};

use tap::Tap;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use super::{
    state::{self, get_state},
    Unit, UnitEntry,
};
use crate::{
    unit::state::{set_state_with_condition, State},
    Rc,
};

type Item = Rc<dyn Unit + Send + Sync + 'static>;

#[derive(Debug)]
pub(crate) struct UnitStore {
    map: HashMap<UnitEntry, Item>, // info in unit files
    state_manager: Sender<state::Message>,
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
    pub(crate) fn new(state_manager: Sender<state::Message>) -> Self {
        Self {
            map: HashMap::new(),
            state_manager,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let entry = msg.0;
                match msg.1 {
                    Action::Update(unit) => {
                        println!("updating unit: {:?}", &entry);
                        self.map.insert(entry, unit);
                    }
                    Action::Remove => {
                        self.map.remove(&entry);
                    }
                    Action::Start => {
                        println!("starting unit: {:?}", &entry);
                        if let Some(unit) = self.map.get(&entry) {
                            // find deps
                            let mut requires = self.find_requires(entry).await;
                            while let Some(unit) = requires.pop() {
                                unit.start(self.state_manager.clone()).await;
                            }
                        }
                    }
                    Action::Stop => todo!(),
                    Action::Restart => todo!(),
                }
            }
        })
    }

    async fn find_requires(&mut self, entry: UnitEntry) -> Vec<Item> {
        let mut queue = VecDeque::new();
        queue.push_back(entry);
        let mut stack = Vec::new();
        while let Some(e) = queue.pop_front() {
            if get_state(&self.state_manager, e.clone())
                .await
                .is_inactive()
            {
                println!("finding requires...");
                if let Some(unit) = self.map.get(&e) {
                    let unit = unit.clone();
                    let deps = unit.deps();
                    for dep in deps.requires.iter().cloned() {
                        println!("pushing {:?} into queue", &dep);
                        queue.push_back(dep);
                    }
                    if stack
                        .iter()
                        .all(|u_in_stack| !Rc::ptr_eq(&unit, u_in_stack))
                    {
                        stack.push(unit);
                    }
                } else {
                    todo!("handle misssing unit dep")
                }
            }
        }
        stack
    }

    fn clear(&mut self) {
        self.map.clear()
    }
}

pub(crate) async fn update_unit(store: &Sender<Message>, unit: impl Unit + Send + Sync + 'static) {
    let entry = UnitEntry::from(&unit);
    store
        .send(Message(entry, Action::Update(Rc::new(unit))))
        .await
        .unwrap();
}

pub(crate) async fn start_unit(store: &Sender<Message>, entry: UnitEntry) {
    store.send(Message(entry, Action::Start)).await.unwrap();
}

// pub struct DepMgr {
//     map: HashMap<UnitEntry, UnitDeps>,
// }
//
// impl DepMgr {
//     pub fn new() -> Self {
//         Self {
//             map: HashMap::new(),
//         }
//     }
//
//     // pub fn insert(&mut self, unit: &dyn Unit) {
//     //     let entry: UnitEntry = unit.into();
//     //     let UnitDeps {
//     //         requires,
//     //         required_by,
//     //     } = unit.deps().clone();
//     //     for unit in &required_by {
//     //         self.map
//     //             .entry(unit.clone())
//     //             .or_default()
//     //             .requires
//     //             .push(entry.clone());
//     //     }
//     //     match self.map.entry(entry) {
//     //         Entry::Occupied(o) => o.into_mut().requires.extend(requires),
//     //         Entry::Vacant(v) => {
//     //             v.insert(UnitDeps {
//     //                 requires,
//     //                 required_by,
//     //             });
//     //         }
//     //     }
//     // }
//
//     pub(crate) fn do_with_deps(
//         &self,
//         unit: UnitEntry,
//         mut action: impl FnMut(&UnitEntry),
//         mut condition: impl FnMut(&UnitEntry) -> bool,
//     ) {
//         if !condition(&unit) {
//             return;
//         }
//         let mut stack = Vec::new();
//         let mut queue = VecDeque::new();
//         queue.push_back(unit);
//         while let Some(current) = queue.pop_front() {
//             if let Some(unit) = self.map.get(&current) {
//                 queue.extend(unit.requires.iter().filter(|u| condition(u)).cloned());
//             }
//             if !stack.contains(&current) {
//                 stack.push(current);
//             }
//         }
//         for unit in stack {
//             action(&unit);
//         }
//     }
// }
