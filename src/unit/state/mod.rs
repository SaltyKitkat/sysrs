use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Display,
};

use tokio::{
    sync::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};

use super::{dep, UnitEntry};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Uninit = 0,
    Stopped,
    Failed,
    Starting,
    Active,
    Stopping,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            State::Uninit => "Uninit",
            State::Stopped => "Stopped",
            State::Failed => "Failed",
            State::Starting => "Starting",
            State::Active => "Active",
            State::Stopping => "Stopping",
        };
        write!(f, "{}", s)
    }
}

impl State {
    pub(crate) fn is_active(&self) -> bool {
        match self {
            State::Uninit | State::Stopped | State::Failed | State::Starting | State::Stopping => {
                false
            }
            State::Active => true,
        }
    }
    pub(crate) fn is_dead(&self) -> bool {
        match self {
            State::Starting | State::Active | State::Stopping => false,
            State::Uninit | State::Stopped | State::Failed => true,
        }
    }
}

type MonitorRet = oneshot::Sender<Result<State, State>>;

#[derive(Debug)]
pub(crate) struct StateManager {
    state: HashMap<UnitEntry, State>,
    monitor: HashMap<UnitEntry, Vec<MonitorRet>>,
    dep: Sender<dep::Message>,
}

pub(crate) enum Message {
    DbgPrint,
    Get(UnitEntry, oneshot::Sender<State>),
    Monitor {
        entry: UnitEntry,
        s: MonitorRet,
        cond: Box<dyn FnOnce(State) -> bool + Send + 'static>,
    },
    Set(UnitEntry, State),
    SetWithCondition {
        entry: UnitEntry,
        new_state: State,
        condition: Box<dyn FnOnce(State) -> bool + Send + 'static>,
    },
}

impl StateManager {
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
