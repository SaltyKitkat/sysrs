use std::collections::{hash_map::Entry, HashMap};

use async_recursion::async_recursion;
use futures::stream::FuturesUnordered;
use tap::Tap;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};
use tokio_stream::StreamExt;

use super::{state, Unit, UnitDeps, UnitEntry};
use crate::{
    unit::state::{register_state_monitor, set_state_with_condition, State},
    util::job,
    Rc,
};

type Item = Rc<dyn Unit + Send + Sync + 'static>;

#[derive(Debug)]
pub(crate) struct UnitStore {
    map: HashMap<UnitEntry, Item>, // info in unit files
    job_manager: Sender<job::Message>,
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
    pub(crate) fn new(
        job_manager: Sender<job::Message>,
        state_manager: Sender<state::Message>,
    ) -> Self {
        Self {
            map: HashMap::new(),
            job_manager,
            state_manager,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                self.serve(msg);
            }
        })
    }

    fn serve(&mut self, msg: Message) {
        let entry = msg.0;
        match msg.1 {
            Action::Update(unit) => self.insert(entry, unit),
            Action::Remove => {
                self.map.remove(&entry);
            }
            Action::Start => todo!(),
            Action::Stop => todo!(),
            Action::Restart => todo!(),
        }
    }

    /// start a unit with its deps
    /// spawn a start job and set unit state to starting
    #[async_recursion]
    async fn start(&self, entry: UnitEntry) {
        if let Some(unit) = self.map.get(&entry) {
            let deps = unit.deps();
            let UnitDeps {
                requires,
                wants,
                after,
                before,
                conflicts,
            } = deps.as_ref();

            // currently we just start all the things we need, wait them started and then start this unit.
            // todo: more detailed way:
            // start requires
            // let requires_fut: FuturesUnordered<_> = requires
            //     .iter()
            //     .map(|entry| self.start(entry.clone()))
            //     .collect();
            // start wants
            // stop conflicts
            // wait conflicts stop
            // wait afters
            // notify befores

            // requires
            let mut requires_fut: FuturesUnordered<_> = requires
                .iter()
                .cloned()
                .map(|e| {
                    let fut = async {
                        self.start(e.clone()).await;
                        register_state_monitor(&self.state_manager, e)
                            .await
                            .await
                            .unwrap()
                    };
                    fut
                })
                .collect();
            if !requires_fut.all(|state| state == State::Running).await {
                todo!("handle dep failure");
            }

            // after
            let mut after_fut: FuturesUnordered<_> = after
                .iter()
                .cloned()
                .map(|e| {
                    let fut = async {
                        self.start(e.clone()).await;
                        register_state_monitor(&self.state_manager, e)
                            .await
                            .await
                            .unwrap()
                    };
                    fut
                })
                .collect();
            if !after_fut.all(|state| state == State::Running).await {
                todo!("handle dep failure");
            }

            // wants
            let mut wants_fut: FuturesUnordered<_> = wants
                .iter()
                .cloned()
                .map(|e| {
                    let fut = async {
                        self.start(e.clone()).await;
                        register_state_monitor(&self.state_manager, e)
                            .await
                            .await
                            .unwrap()
                    };
                    fut
                })
                .collect();
            if !wants_fut.all(|state| state == State::Running).await {
                todo!("handle dep failure");
            }

            // confilcts
            // todo!();

            //  start self
            self._start(entry);
        }
        todo!()
    }
    /// start a single unit
    fn _start(&self, entry: UnitEntry) {
        if let Some(unit) = self.map.get(&entry) {
            let unit = unit.clone();
            let job_manager = self.job_manager.clone();
            let state_manager = self.state_manager.clone();
            // todo: start deps
            tokio::spawn(async move {
                // check previous state and set state to starting
                if let Some(s) =
                    set_state_with_condition(&state_manager, entry, State::Starting, |s| {
                        s == State::Stopped || s == State::Failed
                    })
                    .await
                {
                    match s {
                        Ok(s) => {
                            unit.start(job_manager);
                        }
                        Err(s) => todo!("handle error"),
                    }
                };
                todo!("start a unit")
            });
        } else {
            todo!("handle missing unit")
        }

        // query unit state
        // get deps
        // start(self, deps)
        // tell job manager, which will set the unit state later, to run the start job
        // done
    }

    fn insert(&mut self, entry: UnitEntry, unit: Item) {
        let unit = match self.map.entry(entry.clone()) {
            Entry::Occupied(o) => o.into_mut().tap_mut(|o| **o = unit),
            Entry::Vacant(v) => v.insert(unit),
        };
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

pub(crate) async fn start_unit(store: &Sender<Message>, entry: UnitEntry) -> Result<State, State> {
    store.send(Message(entry, Action::Start)).await.unwrap();
    todo!()
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
