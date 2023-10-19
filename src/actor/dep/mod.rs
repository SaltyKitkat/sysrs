use std::collections::{hash_map::Entry, HashMap, HashSet};

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

use super::{
    guard::{self, is_guard_exists},
    state::{self, get_state},
};

/// runtime mutable dep info, used to wait deps
///
/// befores is useless in DepInfo since it will never block self.
struct DepInfo {
    requires: HashSet<UnitEntry>,
    wants: HashSet<UnitEntry>,
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
            after,
            conflicts,
        } = self;
        requires.is_empty() && wants.is_empty() && after.is_empty() && conflicts.is_empty()
    }
}

// after is useless in ReverseDepInfo since what we want is triggers/blocking_relations here,
// self will never block afters start
#[derive(Default)]
struct ReverseDepInfo {
    required_by: HashSet<UnitEntry>,
    wanted_by: HashSet<UnitEntry>,
    before: HashSet<UnitEntry>,
    conflicts: HashSet<UnitEntry>,
}

impl ReverseDepInfo {
    fn is_empty(&self) -> bool {
        self.required_by.is_empty()
            && self.wanted_by.is_empty()
            && self.before.is_empty()
            && self.conflicts.is_empty()
    }
}

pub(crate) enum Message {
    /// 加载一个Unit的依赖信息
    Load(UnitEntry, Rc<UnitDeps>),
    /// 更新一个Unit的依赖信息
    Update {
        id: UnitEntry,
        old: Rc<UnitDeps>,
        new: Rc<UnitDeps>,
    },
    /// 增加一项等待启动的Unit
    AddToStartList(UnitEntry, Rc<UnitDeps>),
    /// 收到通知事件：指定Unit的状态发生改变
    StateChange(UnitEntry, State),
}
pub(crate) struct DepStore {
    start_list: HashMap<UnitEntry, DepInfo>,
    reverse_map: HashMap<UnitEntry, ReverseDepInfo>,
    state: Sender<state::Message>,
    guard: Sender<guard::Message>,
}

impl DepStore {
    pub(crate) fn new(state: Sender<state::Message>, guard: Sender<guard::Message>) -> Self {
        Self {
            start_list: Default::default(),
            reverse_map: Default::default(),
            state,
            guard,
        }
    }
    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    // todo: implement update deps(load already loaded deps)
                    Message::Load(entry, deps) => {
                        reverse_map_insert(&entry, &deps.requires, &mut self.reverse_map, |rdep| {
                            &mut rdep.required_by
                        });
                        reverse_map_insert(&entry, &deps.wants, &mut self.reverse_map, |rdep| {
                            &mut rdep.wanted_by
                        });
                        reverse_map_insert(&entry, &deps.after, &mut self.reverse_map, |rdep| {
                            &mut rdep.before
                        });
                        reverse_map_insert(
                            &entry,
                            &deps.conflicts,
                            &mut self.reverse_map,
                            |rdep| &mut rdep.conflicts,
                        );
                    }
                    // todo: test
                    Message::Update { id, old, new } => {
                        // remove old
                        reverse_map_remove(&id, &old.requires, &mut self.reverse_map, |rdep| {
                            &mut rdep.required_by
                        });
                        reverse_map_remove(&id, &old.wants, &mut self.reverse_map, |rdep| {
                            &mut rdep.wanted_by
                        });
                        reverse_map_remove(&id, &old.after, &mut self.reverse_map, |rdep| {
                            &mut rdep.before
                        });
                        reverse_map_remove(&id, &old.conflicts, &mut self.reverse_map, |rdep| {
                            &mut rdep.conflicts
                        });
                        // insert new
                        reverse_map_insert(&id, &new.requires, &mut self.reverse_map, |rdep| {
                            &mut rdep.required_by
                        });
                        reverse_map_insert(&id, &new.wants, &mut self.reverse_map, |rdep| {
                            &mut rdep.wanted_by
                        });
                        reverse_map_insert(&id, &new.after, &mut self.reverse_map, |rdep| {
                            &mut rdep.before
                        });
                        reverse_map_insert(&id, &new.conflicts, &mut self.reverse_map, |rdep| {
                            &mut rdep.conflicts
                        });
                    }
                    Message::AddToStartList(entry, deps) => {
                        // since there's already a dep in here waiting for its deps
                        // dont need to insert another time
                        if self.start_list.contains_key(&entry) {
                            continue;
                        }
                        let mut dep_info = DepInfo::from(deps.as_ref());
                        // remove already active units int the dep_info list
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
                        dep_info.after = stream::iter(dep_info.after)
                            .filter(|after| {
                                let after = after.clone();
                                async { !get_state(&self.state, after).await.is_active() }
                            })
                            .collect()
                            .await;
                        dep_info.conflicts = stream::iter(dep_info.conflicts)
                            .filter(|conflict| {
                                let conflict = conflict.clone();
                                async { is_guard_exists(&self.guard, conflict).await }
                            })
                            .collect()
                            .await;

                        if dep_info.can_start() {
                            self.guard
                                .send(guard::Message::DepsReady(entry))
                                .await
                                .unwrap();
                        } else {
                            self.start_list.insert(entry, dep_info);
                        }
                    }
                    Message::StateChange(entry, new_state) => {
                        let Self {
                            start_list: map,
                            reverse_map,
                            state,
                            guard,
                        } = &mut self;
                        if let Entry::Occupied(reverse_dep) = reverse_map.entry(entry.clone()) {
                            let reverse_dep = reverse_dep.get();
                            match new_state {
                                State::Uninit => unreachable!(),
                                State::Stopped => {
                                    for unit in reverse_dep.before.iter() {
                                        if reverse_dep.conflicts.contains(unit) {
                                            let dep = map.get_mut(unit).unwrap();
                                            dep.after.remove(&entry);
                                            if dep.can_start() {
                                                map.remove(unit);
                                                guard
                                                    .send(guard::Message::DepsReady(unit.clone()))
                                                    .await
                                                    .unwrap();
                                            }
                                        }
                                    }
                                }
                                State::Failed => {
                                    for unit in reverse_dep.required_by.iter() {
                                        map.remove(unit);
                                        guard
                                            .send(guard::Message::DepsFailed(unit.clone()))
                                            .await
                                            .unwrap()
                                    }
                                }
                                State::Starting => {
                                    for unit in reverse_dep.required_by.iter() {
                                        let dep = map.get_mut(unit).unwrap();
                                        dep.requires.remove(&entry);
                                        if dep.can_start() {
                                            map.remove(unit);
                                            guard
                                                .send(guard::Message::DepsReady(unit.clone()))
                                                .await
                                                .unwrap();
                                        }
                                    }
                                    for unit in reverse_dep.wanted_by.iter() {
                                        let dep = map.get_mut(unit).unwrap();
                                        dep.wants.remove(&entry);
                                        if dep.can_start() {
                                            map.remove(unit);
                                            guard
                                                .send(guard::Message::DepsReady(unit.clone()))
                                                .await
                                                .unwrap();
                                        }
                                    }
                                }
                                State::Active => {
                                    for unit in reverse_dep.before.iter() {
                                        if reverse_dep.required_by.contains(unit)
                                            || reverse_dep.wanted_by.contains(unit)
                                        {
                                            let dep = map.get_mut(unit).unwrap();
                                            dep.after.remove(&entry);
                                            if dep.can_start() {
                                                map.remove(unit);
                                                guard
                                                    .send(guard::Message::DepsReady(unit.clone()))
                                                    .await
                                                    .unwrap();
                                            }
                                        }
                                    }
                                }
                                // stop `required_by`s and remove conflicts
                                State::Stopping => {
                                    for unit in reverse_dep.required_by.iter() {
                                        guard
                                            .send(guard::Message::Stop(unit.clone()))
                                            .await
                                            .unwrap()
                                    }
                                    for unit in reverse_dep.conflicts.iter() {
                                        let dep = map.get_mut(unit).unwrap();
                                        dep.conflicts.remove(&entry);
                                        if dep.can_start() {
                                            map.remove(unit);
                                            guard
                                                .send(guard::Message::DepsReady(unit.clone()))
                                                .await
                                                .unwrap();
                                        }
                                    }
                                }
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

fn reverse_map_remove(
    unit0: &UnitEntry,
    src: &[UnitEntry],
    target: &mut HashMap<UnitEntry, ReverseDepInfo>,
    field: impl Fn(&mut ReverseDepInfo) -> &mut HashSet<UnitEntry>,
) {
    for unit in src.iter() {
        if let Some(item) = target.get_mut(unit) {
            item.pipe(&field).remove(unit0);
        }
    }
}
