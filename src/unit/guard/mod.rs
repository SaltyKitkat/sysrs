use std::collections::HashMap;

use futures::{future::BoxFuture, Future};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

use super::{
    state::{self, set_state, State},
    store, UnitEntry,
};

pub(crate) enum Message {
    /// Insert a guard.
    Insert(
        UnitEntry,
        Box<
            dyn FnOnce(
                    Sender<store::Message>,
                    Sender<state::Message>,
                    Receiver<GuardMessage>,
                ) -> BoxFuture<'static, State>
                + Send
                + 'static,
        >, // todo: guard refactor
    ),
    /// remove a guard \
    /// usually called by self when a gurad quits
    Remove(UnitEntry),
    /// notice all deps are ready for a specific unit \
    /// called by `Dep`
    DepsReady(UnitEntry),
    /// Send a Stop message to the specific unit guard
    Stop(UnitEntry),
}

#[derive(Debug, Clone)]
pub(crate) struct GuardManager {
    map: HashMap<UnitEntry, Sender<GuardMessage>>,
    self_: Sender<Message>,
    store: Sender<store::Message>,
    state: Sender<state::Message>,
}

impl GuardManager {
    pub(crate) fn new(
        self_: Sender<Message>,
        store: Sender<store::Message>,
        state: Sender<state::Message>,
    ) -> Self {
        Self {
            map: HashMap::new(),
            self_,
            store,
            state,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Insert(u, s) => {
                        let (sender, recevier) = mpsc::channel(4); // todo: remove magic number
                        let fut = s(self.store.clone(), self.state.clone(), recevier);
                        self.map.insert(u.clone(), sender); // todo: should return None, deal with Some(_) cases
                        let sender = self.self_.clone();
                        let state = self.state.clone();
                        tokio::spawn(async move {
                            let new_state = fut.await;
                            // remove the entry after the guard end
                            set_state(&state, u.clone(), new_state).await;
                            sender.send(Message::Remove(u)).await.unwrap();
                        });
                    }
                    Message::Remove(u) => {
                        self.map.remove(&u);
                    }
                    Message::DepsReady(u) => {
                        self.map
                            .get(&u)
                            .unwrap()
                            .send(GuardMessage::DepsReady)
                            .await
                            .unwrap();
                    }
                    Message::Stop(u) => {
                        self.map
                            .get(&u)
                            .unwrap()
                            .send(GuardMessage::Stop)
                            .await
                            .unwrap();
                    }
                }
            }
        })
    }
}

pub(crate) enum GuardMessage {
    DepsReady,
    Stop,
}

// todo: guard refactor
pub(crate) async fn create_guard<F, Fut>(guard_manager: &Sender<Message>, u: UnitEntry, f: F)
where
    F: FnOnce(Sender<store::Message>, Sender<state::Message>, Receiver<GuardMessage>) -> Fut
        + Send
        + 'static,
    Fut: Future<Output = State> + Send + 'static,
{
    guard_manager
        .send(Message::Insert(
            u,
            Box::new(|store, state, rx| Box::pin(f(store, state, rx))),
        ))
        .await
        .unwrap();
}

pub(crate) async fn guard_stop(guard_manager: &Sender<Message>, u: UnitEntry) {
    guard_manager.send(Message::Stop(u)).await.unwrap()
}
