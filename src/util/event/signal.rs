use futures::{Future, StreamExt};
use rustix::process::{wait, WaitOptions};
use tokio::signal::unix::{signal, SignalKind};
use tokio_stream::wrappers::SignalStream;

pub(crate) fn register_sig_handlers() {
    // handle ctrl-c/SIGINT
    register_signal_handler(SignalKind::interrupt(), |mut stream| async move {
        while let Some(_) = stream.next().await {
            println!("SIGINT!")
        }
    });
    // handle SIGCHLD
    register_signal_handler(SignalKind::child(), |mut stream| async move {
        while let Some(_) = stream.next().await {
            match wait(WaitOptions::NOHANG) {
                Ok(Some((pid, status))) => todo!(), // interact with the monitor
                Ok(None) => todo!(),
                Err(e) => todo!(),
            }
        }
    })
}

fn register_signal_handler<F, H>(signalkind: SignalKind, handler: H)
where
    F: Future<Output = ()> + Send + 'static,
    H: FnOnce(SignalStream) -> F,
{
    let sig = SignalStream::new(signal(signalkind).unwrap());
    tokio::spawn(handler(sig));
}
