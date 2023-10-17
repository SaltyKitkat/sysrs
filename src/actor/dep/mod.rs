use std::collections::{HashMap, HashSet};

use futures_util::{stream, StreamExt};
use tap::Pipe;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use crate::{
    unit::{State, UnitDeps, UnitEntry},
    Rc,
};

use super::guard::{self, is_guard_exists};

/// runtime mutable dep info, used to wait deps
struct DepInfo {
    requires: HashSet<UnitEntry>,
    wants: HashSet<UnitEntry>,
    before: HashSet<UnitEntry>,
    after: HashSet<UnitEntry>,
    conflicts: HashSet<UnitEntry>,
}
impl From<&UnitDeps> for DepInfo {
    fn from(value: &UnitDeps) -> Self {
        let UnitDeps {
            requires,
            wants,
            after,
            before,
            conflicts,
        } = value;
        Self {
            requires: requires.iter().cloned().collect(),
            wants: wants.iter().cloned().collect(),
            before: before.iter().cloned().collect(),
            after: after.iter().cloned().collect(),
            conflicts: conflicts.iter().cloned().collect(),
        }
    }
}
impl DepInfo {
    fn can_start(&self) -> bool {
        let Self {
            requires,
            wants,
            before,
            after,
            conflicts,
        } = self;
        requires.is_empty() && wants.is_empty() && after.is_empty()
    }
}

#[derive(Default)]
struct ReverseDepInfo {
    required_by: HashSet<UnitEntry>,
    wanted_by: HashSet<UnitEntry>,
    before: HashSet<UnitEntry>,
    after: HashSet<UnitEntry>,
    conflicts: HashSet<UnitEntry>,
}

pub(crate) enum Message {
    /// 增加一项等待启动的Unit
    Insert(UnitEntry, Rc<UnitDeps>),
    /// 收到通知事件：指定Unit的状态发生改变
    StateChange(UnitEntry, State),
}
pub(crate) struct Dep {
    map: HashMap<UnitEntry, DepInfo>,
    reverse_map: HashMap<UnitEntry, ReverseDepInfo>,
    guard: Sender<guard::Message>,
}

impl Dep {
    pub(crate) fn new(guard: Sender<guard::Message>) -> Self {
        Self {
            map: Default::default(),
            reverse_map: Default::default(),
            guard,
        }
    }
    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Insert(entry, deps) => {
                        let mut dep_info = DepInfo::from(deps.as_ref());
                        // todo: remove already active units int the dep_info list
                        dep_info.wants = stream::iter(dep_info.wants)
                            .filter(|want| {
                                let want = want.clone();
                                async { !is_guard_exists(&self.guard, want).await }
                            })
                            .collect()
                            .await;
                        dep_info.requires = stream::iter(dep_info.requires)
                            .filter(|require| {
                                let require = require.clone();
                                async { !is_guard_exists(&self.guard, require).await }
                            })
                            .collect()
                            .await;
                        if dep_info.can_start() {
                            self.guard
                                .send(guard::Message::DepsReady(entry))
                                .await
                                .unwrap();
                        } else {
                            reverse_map_insert(
                                &entry,
                                &deps.requires,
                                &mut self.reverse_map,
                                |dep| &mut dep.required_by,
                            );
                            reverse_map_insert(&entry, &deps.wants, &mut self.reverse_map, |dep| {
                                &mut dep.wanted_by
                            });
                            reverse_map_insert(&entry, &deps.after, &mut self.reverse_map, |dep| {
                                &mut dep.before
                            });
                            reverse_map_insert(
                                &entry,
                                &deps.before,
                                &mut self.reverse_map,
                                |dep| &mut dep.after,
                            );
                            reverse_map_insert(
                                &entry,
                                &deps.conflicts,
                                &mut self.reverse_map,
                                |dep| &mut dep.conflicts,
                            );
                            self.map.insert(entry, dep_info);
                        }
                    }
                    Message::StateChange(entry, new_state) => {
                        let Self {
                            map,
                            reverse_map,
                            guard,
                        } = &mut self;
                        if let Some(reverse_dep) = reverse_map.get_mut(&entry) {
                            match new_state {
                                State::Uninit => todo!(),
                                State::Stopped => todo!(),
                                State::Failed => todo!(),
                                State::Starting => {
                                    for unit in reverse_dep.required_by.drain() {
                                        let dep = map.get_mut(&unit).unwrap();
                                        dep.requires.remove(&entry);
                                        if dep.can_start() {
                                            map.remove(&unit);
                                            guard
                                                .send(guard::Message::DepsReady(unit))
                                                .await
                                                .unwrap();
                                        }
                                    }
                                    for unit in reverse_dep.wanted_by.drain() {
                                        let dep = map.get_mut(&unit).unwrap();
                                        dep.wants.remove(&entry);
                                        if dep.can_start() {
                                            map.remove(&unit);
                                            guard
                                                .send(guard::Message::DepsReady(unit))
                                                .await
                                                .unwrap();
                                        }
                                    }
                                }
                                State::Active => {
                                    for unit in reverse_dep.after.drain() {
                                        let dep = map.get_mut(&unit).unwrap();
                                        dep.requires.remove(&entry);
                                        if dep.can_start() {
                                            map.remove(&unit);
                                            guard
                                                .send(guard::Message::DepsReady(unit))
                                                .await
                                                .unwrap();
                                        }
                                    }
                                }
                                State::Stopping => todo!(),
                            }
                        }
                    }
                }
            }
        })
    }
}

fn reverse_map_insert(
    unit0: &UnitEntry,
    src: &[UnitEntry],
    target: &mut HashMap<UnitEntry, ReverseDepInfo>,
    field: impl Fn(&mut ReverseDepInfo) -> &mut HashSet<UnitEntry>,
) {
    for unit in src.iter() {
        target
            .entry(unit.clone())
            .or_default()
            .pipe(&field)
            .insert(unit0.clone());
    }
}
