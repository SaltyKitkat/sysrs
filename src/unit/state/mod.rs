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

use super::UnitEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Uninit,
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
    pub(crate) fn is_inactive(&self) -> bool {
        match self {
            State::Starting | State::Active | State::Stopping => false,
            State::Uninit | State::Stopped | State::Failed => true,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct StateManager {
    state: HashMap<UnitEntry, State>,
    monitor: HashMap<UnitEntry, Vec<oneshot::Sender<State>>>,
}

pub(crate) enum Action {
    Get(oneshot::Sender<State>),
    Monitor(oneshot::Sender<State>),
    Set(State),
    SetWithCondition {
        target: State,
        condition: Box<dyn FnOnce(State) -> bool + Send + 'static>,
    },
}
pub(crate) struct Message(UnitEntry, Action);

impl StateManager {
    pub(crate) fn new() -> Self {
        Default::default()
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
            Action::Get(s) => {
                if let Some(&state) = self.state.get(&entry) {
                    s.send(state).ok();
                } else {
                    s.send(State::Uninit).ok();
                }
            }
            Action::Monitor(s) => match self.monitor.entry(entry) {
                Entry::Occupied(mut o) => {
                    o.get_mut().push(s);
                }
                Entry::Vacant(v) => {
                    v.insert(vec![s]);
                }
            },
            Action::Set(new_state) => self.set(entry, new_state),
            Action::SetWithCondition {
                target: new_state,
                condition,
            } => {
                let old_state = self.state.get(&entry).unwrap_or(&State::Uninit);
                if condition(*old_state) {
                    self.set(entry, new_state);
                }
            }
        }
    }

    fn set(&mut self, entry: UnitEntry, state: State) {
        println!("setting state: `{entry}` to `{state}`");
        self.trigger_monitors(&entry, state);
        self.state.insert(entry, state);
    }

    fn trigger_monitors(&mut self, entry: &UnitEntry, new_state: State) {
        if let Some(monitors) = self.monitor.remove(entry) {
            for monitor in monitors {
                monitor.send(new_state).ok();
            }
        }
    }
}

pub(crate) async fn get_state(state_manager: &Sender<Message>, entry: UnitEntry) -> State {
    let (s, r) = oneshot::channel();
    state_manager
        .send(Message(entry, Action::Get(s)))
        .await
        .unwrap();
    r.await.unwrap()
}

pub(crate) async fn set_state(state_manager: &Sender<Message>, entry: UnitEntry, state: State) {
    state_manager
        .send(Message(entry, Action::Set(state)))
        .await
        .unwrap();
}

/// check the current state. if fit the condition, set the state to target.
/// return the previous state.
pub(crate) async fn set_state_with_condition(
    state_manager: &Sender<Message>,
    entry: UnitEntry,
    target: State,
    condition: impl FnOnce(State) -> bool + Send + 'static,
) -> Result<State, State> {
    // hook: add oneshot in condition closure
    // and get the previous state
    let (s, r) = oneshot::channel();
    state_manager
        .send(Message(
            entry,
            Action::SetWithCondition {
                target,
                condition: Box::new(|state| {
                    let ret = condition(state);
                    let state = if ret { Ok(state) } else { Err(state) };
                    s.send(state).unwrap();
                    ret
                }),
            },
        ))
        .await
        .unwrap();
    r.await.unwrap()
}

pub(crate) async fn register_state_monitor(
    state_manager: &Sender<Message>,
    entry: UnitEntry,
) -> oneshot::Receiver<State> {
    let (s, r) = oneshot::channel();
    state_manager
        .send(Message(entry, Action::Monitor(s)))
        .await
        .unwrap();
    r
}
