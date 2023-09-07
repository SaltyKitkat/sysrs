use futures::Future;
use rustix::process::{wait, WaitOptions};
use tokio::{
    signal::unix::{signal, Signal, SignalKind},
    sync::{mpsc::Sender, Notify},
    task::JoinHandle,
};

use crate::{util::monitor::Message, Actors, Rc};

/// all the posix sig habdlers should be registered here
/// should be called under tokio rt
pub(crate) fn register_sig_handlers(actors: &Actors) {
    // handle ctrl-c/SIGINT
    register_signal_handler(SignalKind::interrupt(), |mut signal| async move {
        loop {
            signal.recv().await;
            println!("SIGINT!");
        }
    });
    // handle SIGCHLD
    let monitor = actors.monitor.clone();
    register_signal_handler(SignalKind::child(), |mut signal| async move {
        let n = Rc::new(Notify::new());
        let w = ChldWaiter::new(monitor).run(n.clone());
        loop {
            signal.recv().await;
            n.notify_waiters();
        }
    })
}

/// wait child processes until there's no more zombies.
/// triggered by sigchld.
struct ChldWaiter {
    monitor: Sender<Message>,
}
impl ChldWaiter {
    fn new(monitor: Sender<Message>) -> Self {
        Self { monitor }
    }
    fn run(self, rx: Rc<Notify>) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                rx.notified().await;
                loop {
                    match wait(WaitOptions::NOHANG) {
                        Ok(Some(i)) => {
                            self.monitor.send(Message::Process(i)).await.unwrap();
                        }
                        Ok(None) => break,
                        Err(e) => todo!("handle error"),
                    }
                }
            }
        })
    }
}

fn register_signal_handler<F, H>(signalkind: SignalKind, handler: H)
where
    F: Future<Output = ()> + Send + 'static,
    H: FnOnce(Signal) -> F,
{
    let sig = signal(signalkind).unwrap();
    tokio::spawn(handler(sig));
}
