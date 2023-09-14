use std::collections::{hash_map::Entry, HashMap};

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
    Running,
    Stopping,
}

#[derive(Debug, Default)]
pub(crate) struct StateManager {
    state: HashMap<UnitEntry, State>,
    monitor: HashMap<UnitEntry, Vec<oneshot::Sender<State>>>,
}
pub(crate) enum Action {
    Set(State),
    Get(oneshot::Sender<State>),
    SetWithCondition {
        target: State,
        condition: Box<dyn FnOnce(State) -> bool + Send + 'static>,
    },
    Monitor(oneshot::Sender<State>),
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
            Action::Set(new_state) => {
                self.trigger_monitors(&entry, new_state);
                self.state.insert(entry, new_state);
            }
            Action::Get(s) => {
                if let Some(&state) = self.state.get(&entry) {
                    s.send(state).ok();
                }
            }
            Action::SetWithCondition { target, condition } => {
                if let Entry::Occupied(mut e) = self.state.entry(entry.clone()) {
                    if condition(e.get().clone()) {
                        *e.get_mut() = target;
                        self.trigger_monitors(&entry, target);
                    }
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
        }
    }

    fn trigger_monitors(&mut self, entry: &UnitEntry, new_state: State) {
        if let Some(monitors) = self.monitor.remove(entry) {
            for monitor in monitors {
                monitor.send(new_state).ok();
            }
        }
    }
}

pub(crate) async fn get_state(state_manager: &Sender<Message>, entry: UnitEntry) -> Option<State> {
    let (s, r) = oneshot::channel();
    state_manager
        .send(Message(entry, Action::Get(s)))
        .await
        .unwrap();
    r.await.ok()
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
) -> Option<Result<State, State>> {
    // hook: add oneshot in condition closure
    // if condition called, the unit exists, then get the state
    // else return none
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
    r.await.ok().or(None)
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
