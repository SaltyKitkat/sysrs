use futures::Future;
use tokio::signal::unix::{signal, Signal, SignalKind};

use crate::Actors;

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
}

fn register_signal_handler<F, H>(signalkind: SignalKind, handler: H)
where
    F: Future<Output = ()> + Send + 'static,
    H: FnOnce(Signal) -> F,
{
    let sig = signal(signalkind).unwrap();
    tokio::spawn(handler(sig));
}
