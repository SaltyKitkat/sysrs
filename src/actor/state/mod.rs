use std::collections::{hash_map::Entry, HashMap};

use tokio::{
    sync::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};

use super::dep;
use crate::unit::{State, UnitEntry};

type MonitorRet = oneshot::Sender<Result<State, State>>;

pub(crate) enum Message {
    /// 打印内部信息，用于调试
    DbgPrint,
    /// 获得指定Unit的状态
    Get(UnitEntry, oneshot::Sender<State>),
    /// 注册一个hook,用于监听特性unit的状态改变 \
    /// 是一个坏的api：由于unit start之后，set state的时机无法确定， \
    ///     因此想要在start一类操作之后获得state作为结果的情景无法使用此api实现
    Monitor {
        entry: UnitEntry,
        s: MonitorRet,
        cond: Box<dyn FnOnce(State) -> bool + Send + 'static>,
    },
    /// 无条件设置指定Unit的状态
    Set(UnitEntry, State),
    /// 以当前状态作为条件决定是否设置指定Unit状态 \
    /// 一定程度上相当于对指定Unit的状态进行CAS原子操作
    SetWithCondition {
        entry: UnitEntry,
        new_state: State,
        condition: Box<dyn FnOnce(State) -> bool + Send + 'static>,
    },
}

#[derive(Debug)]
pub(crate) struct StateStore {
    state: HashMap<UnitEntry, State>,
    monitor: HashMap<UnitEntry, Vec<MonitorRet>>,
    dep: Sender<dep::Message>,
}

impl StateStore {
    pub(crate) fn new(dep: Sender<dep::Message>) -> Self {
        Self {
            state: Default::default(),
            monitor: Default::default(),
            dep,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let ref mut this = self;
                match msg {
                    Message::DbgPrint => println!("{:#?}", this.state),
                    Message::Get(entry, s) => {
                        if let Some(&state) = this.state.get(&entry) {
                            s.send(state).ok();
                        } else {
                            s.send(State::Uninit).ok();
                        }
                    }
                    Message::Monitor { entry, s, cond } => {
                        let state = this.state.get(&entry).copied().unwrap_or_default();
                        if cond(state) {
                            match this.monitor.entry(entry) {
                                Entry::Occupied(mut o) => {
                                    o.get_mut().push(s);
                                }
                                Entry::Vacant(v) => {
                                    v.insert(vec![s]);
                                }
                            }
                        } else {
                            s.send(Err(state)).unwrap();
                        }
                    }
                    Message::Set(entry, new_state) => this.set(entry, new_state).await,
                    Message::SetWithCondition {
                        entry,
                        new_state,
                        condition,
                    } => {
                        let old_state = this.state.get(&entry).unwrap_or(&State::Uninit);
                        if condition(*old_state) {
                            this.set(entry, new_state).await;
                        }
                    }
                }
            }
        })
    }

    /// 设置unit状态时统一使用此api,以触发monitor
    async fn set(&mut self, entry: UnitEntry, state: State) {
        println!("setting state: `{entry}` to `{state}`");
        self.trigger_monitors(&entry, state);
        self.state.insert(entry.clone(), state);
        self.dep
            .send(dep::Message::StateChange(entry, state))
            .await
            .unwrap()
    }

    fn trigger_monitors(&mut self, entry: &UnitEntry, new_state: State) {
        if let Some(monitors) = self.monitor.remove(entry) {
            for monitor in monitors {
                monitor.send(Ok(new_state)).ok();
            }
        }
    }
}

pub(crate) async fn get_state(state_manager: &Sender<Message>, entry: UnitEntry) -> State {
    let (s, r) = oneshot::channel();
    state_manager.send(Message::Get(entry, s)).await.unwrap();
    r.await.unwrap()
}

pub(crate) async fn set_state(state_manager: &Sender<Message>, entry: UnitEntry, state: State) {
    state_manager
        .send(Message::Set(entry, state))
        .await
        .unwrap();
}

/// check the current state. if fit the condition, set the state to target.
/// return the previous state.
pub(crate) async fn set_state_with_condition(
    state_manager: &Sender<Message>,
    entry: UnitEntry,
    new_state: State,
    condition: impl FnOnce(State) -> bool + Send + 'static,
) -> Result<State, State> {
    // hook: add oneshot in condition closure
    // and get the previous state
    let (s, r) = oneshot::channel();
    state_manager
        .send(Message::SetWithCondition {
            entry,
            new_state,
            condition: Box::new(|state| {
                let ret = condition(state);
                let state = if ret { Ok(state) } else { Err(state) };
                s.send(state).unwrap();
                ret
            }),
        })
        .await
        .unwrap();
    r.await.unwrap()
}

pub(crate) async fn register_state_monitor(
    state_manager: &Sender<Message>,
    entry: UnitEntry,
    cond: impl FnOnce(State) -> bool + Send + 'static,
) -> oneshot::Receiver<Result<State, State>> {
    let (s, r) = oneshot::channel();
    state_manager
        .send(Message::Monitor {
            entry,
            s,
            cond: Box::new(cond),
        })
        .await
        .unwrap();
    r
}

pub(crate) async fn print_state(state_manager: &Sender<Message>) {
    state_manager.send(Message::DbgPrint).await.unwrap();
}
