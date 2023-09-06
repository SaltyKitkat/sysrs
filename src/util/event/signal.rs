use futures::Future;
use rustix::process::{wait, WaitOptions};
use tokio::signal::unix::{signal, Signal, SignalKind};

use crate::{util::monitor, Actors};

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
        loop {
            signal.recv().await;
            match wait(WaitOptions::NOHANG) {
                Ok(Some((pid, status))) => {
                    monitor.send(monitor::Message {}).await.unwrap();
                } // interact with the monitor
                Ok(None) => unreachable!("since we've reveived sigchld, this should not be none"),
                Err(e) => todo!("handle error"),
            }
        }
    })
}

fn register_signal_handler<F, H>(signalkind: SignalKind, handler: H)
where
    F: Future<Output = ()> + Send + 'static,
    H: FnOnce(Signal) -> F,
{
    let sig = signal(signalkind).unwrap();
    tokio::spawn(handler(sig));
}
