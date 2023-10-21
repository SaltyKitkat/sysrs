use std::collections::{hash_map::Entry, HashMap, HashSet};

use futures_util::{stream, StreamExt};
use tap::Pipe;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use crate::{
    unit::{State, UnitDeps, UnitId},
    Rc,
};

use super::{
    guard,
    state::{self, get_state},
};

/// runtime mutable dep info, used to wait deps
///
/// we only want to know what is blocking this unit to start
/// so only `after` is useful
struct DepInfo {
    after: HashSet<UnitId>,
}
impl From<&UnitDeps> for DepInfo {
    fn from(value: &UnitDeps) -> Self {
        let UnitDeps { after, .. } = value;
        Self {
            after: after.iter().cloned().collect(),
        }
    }
}
impl DepInfo {
    fn can_start(&self) -> bool {
        self.after.is_empty() //&& requires.is_empty() && wants.is_empty() && conflicts.is_empty()
    }
}

// after is useless in ReverseDepInfo since what we want is triggers/blocking_relations here,
// self will never block afters start
#[derive(Default)]
struct ReverseDepInfo {
    required_by: HashSet<UnitId>,
    wanted_by: HashSet<UnitId>,
    before: HashSet<UnitId>,
    conflicts: HashSet<UnitId>,
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
    Load(UnitId, Rc<UnitDeps>),
    /// 更新一个Unit的依赖信息
    Update {
        id: UnitId,
        old: Rc<UnitDeps>,
        new: Rc<UnitDeps>,
    },
    /// 增加一项等待启动的Unit
    AddToStartList(UnitId, Rc<UnitDeps>),
    /// 收到通知事件：指定Unit的状态发生改变
    StateChange(UnitId, State),
}
pub(crate) struct DepStore {
    start_list: HashMap<UnitId, DepInfo>,
    reverse_map: HashMap<UnitId, ReverseDepInfo>,
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
                    Message::Load(id, deps) => {
                        let rmap = &mut self.reverse_map;
                        reverse_map_insert(&id, &deps.requires, rmap, |rdep| &mut rdep.required_by);
                        reverse_map_insert(&id, &deps.wants, rmap, |rdep| &mut rdep.wanted_by);
                        reverse_map_insert(&id, &deps.after, rmap, |rdep| &mut rdep.before);
                        reverse_map_insert(&id, &deps.conflicts, rmap, |rdep| &mut rdep.conflicts);
                    }
                    // todo: test
                    Message::Update { id, old, new } => {
                        let rmap = &mut self.reverse_map;
                        // remove old
                        reverse_map_remove(&id, &old.requires, rmap, |rdep| &mut rdep.required_by);
                        reverse_map_remove(&id, &old.wants, rmap, |rdep| &mut rdep.wanted_by);
                        reverse_map_remove(&id, &old.after, rmap, |rdep| &mut rdep.before);
                        reverse_map_remove(&id, &old.conflicts, rmap, |rdep| &mut rdep.conflicts);
                        // insert new
                        reverse_map_insert(&id, &new.requires, rmap, |rdep| &mut rdep.required_by);
                        reverse_map_insert(&id, &new.wants, rmap, |rdep| &mut rdep.wanted_by);
                        reverse_map_insert(&id, &new.after, rmap, |rdep| &mut rdep.before);
                        reverse_map_insert(&id, &new.conflicts, rmap, |rdep| &mut rdep.conflicts);
                    }
                    Message::AddToStartList(id, deps) => {
                        // since there's already a dep in here waiting for its deps
                        // dont need to insert another time
                        if self.start_list.contains_key(&id) {
                            continue;
                        }
                        let mut dep_info = DepInfo::from(deps.as_ref());
                        // remove already active units int the dep_info list
                        dep_info.after = stream::iter(dep_info.after)
                            .filter(|after| {
                                let after = after.clone();
                                async { !get_state(&self.state, after).await.is_active() }
                            })
                            .collect()
                            .await;

                        if dep_info.can_start() {
                            self.guard
                                .send(guard::Message::DepsReady(id))
                                .await
                                .unwrap();
                        } else {
                            self.start_list.insert(id, dep_info);
                        }
                    }
                    Message::StateChange(id, new_state) => {
                        let Self {
                            start_list,
                            reverse_map,
                            state,
                            guard,
                        } = &mut self;
                        if let Entry::Occupied(reverse_dep) = reverse_map.entry(id.clone()) {
                            let reverse_dep = reverse_dep.get();
                            match new_state {
                                State::Uninit => unreachable!(),
                                State::Stopped => {
                                    for unit in &reverse_dep.before & &reverse_dep.conflicts {
                                        if let Entry::Occupied(mut o) =
                                            start_list.entry(unit.clone())
                                        {
                                            o.get_mut().after.remove(&id);
                                            if o.get().can_start() {
                                                o.remove();
                                                guard
                                                    .send(guard::Message::DepsReady(unit))
                                                    .await
                                                    .unwrap();
                                            }
                                        }
                                    }
                                }
                                State::Failed => {
                                    for unit in reverse_dep.required_by.iter().cloned() {
                                        start_list.remove(&unit);
                                        guard.send(guard::Message::DepsFailed(unit)).await.unwrap()
                                    }
                                }
                                // notice conflicts to stop
                                State::Starting => {
                                    // is this necessary?
                                    for unit in reverse_dep.conflicts.iter().cloned() {
                                        guard.send(guard::Message::Stop(unit)).await.unwrap();
                                    }
                                }
                                State::Active => {
                                    for unit in &reverse_dep.before
                                        & &(&reverse_dep.required_by | &reverse_dep.wanted_by)
                                    {
                                        if let Entry::Occupied(mut o) =
                                            start_list.entry(unit.clone())
                                        {
                                            o.get_mut().after.remove(&id);
                                            if o.get().can_start() {
                                                o.remove();
                                                guard
                                                    .send(guard::Message::DepsReady(unit))
                                                    .await
                                                    .unwrap();
                                            }
                                        }
                                    }
                                }
                                // stop `required_by`s
                                State::Stopping => {
                                    for unit in reverse_dep.required_by.iter().cloned() {
                                        guard.send(guard::Message::Stop(unit)).await.unwrap()
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
    unit0: &UnitId,
    src: &[UnitId],
    target: &mut HashMap<UnitId, ReverseDepInfo>,
    field: impl Fn(&mut ReverseDepInfo) -> &mut HashSet<UnitId>,
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
    unit0: &UnitId,
    src: &[UnitId],
    target: &mut HashMap<UnitId, ReverseDepInfo>,
    field: impl Fn(&mut ReverseDepInfo) -> &mut HashSet<UnitId>,
) {
    for unit in src.iter() {
        if let Some(item) = target.get_mut(unit) {
            item.pipe(&field).remove(unit0);
        }
    }
}
