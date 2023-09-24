use std::collections::HashMap;

use futures::{future::BoxFuture, Future};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

use super::{state, store, UnitEntry};

#[derive(Debug, Clone)]
pub(crate) struct GuardManager {
    map: HashMap<UnitEntry, Sender<GMessage>>,
    self_: Sender<Message>,
    store: Sender<store::Message>,
    state: Sender<state::Message>,
}

pub(crate) enum Message {
    Update(
        UnitEntry,
        Box<
            dyn FnOnce(
                    Sender<store::Message>,
                    Sender<state::Message>,
                    Receiver<GMessage>,
                ) -> BoxFuture<'static, ()>
                + Send
                + 'static,
        >,
    ),
    Remove(UnitEntry),
    Stop(UnitEntry),
    Kill(UnitEntry),
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
                    Message::Update(u, s) => {
                        let (sender, recevier) = mpsc::channel(4);
                        let f = s(self.store.clone(), self.state.clone(), recevier);
                        self.map.insert(u.clone(), sender);
                        let sender = self.self_.clone();
                        tokio::spawn(async move {
                            f.await;
                            // remove the entry after the guard end
                            sender.send(Message::Remove(u)).await.unwrap();
                        });
                    }
                    Message::Remove(u) => {
                        self.map.remove(&u);
                    }
                    Message::Stop(u) => {
                        self.map
                            .get(&u)
                            .unwrap()
                            .send(GMessage::Stop)
                            .await
                            .unwrap();
                    }
                    Message::Kill(u) => self
                        .map
                        .get(&u)
                        .unwrap()
                        .send(GMessage::Kill)
                        .await
                        .unwrap(),
                }
            }
        })
    }
}

pub(crate) enum GMessage {
    Stop,
    Kill,
}

pub(crate) async fn create_guard<F, Fut>(guard_manager: &Sender<Message>, u: UnitEntry, f: F)
where
    F: FnOnce(Sender<store::Message>, Sender<state::Message>, Receiver<GMessage>) -> Fut
        + Send
        + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    guard_manager
        .send(Message::Update(
            u,
            Box::new(|store, state, rx| Box::pin(f(store, state, rx))),
        ))
        .await
        .unwrap();
}

pub(crate) async fn guard_stop(guard_manager: &Sender<Message>, u: UnitEntry) {
    guard_manager.send(Message::Stop(u)).await.unwrap()
}

pub(crate) async fn guard_kill(guard_manager: &Sender<Message>, u: UnitEntry) {
    guard_manager.send(Message::Stop(u)).await.unwrap()
}
