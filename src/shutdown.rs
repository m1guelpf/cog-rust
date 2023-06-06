use axum::Extension;
use std::{
    error::Error,
    fmt,
    fmt::Display,
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
};
use tokio::{signal, sync::mpsc};

#[derive(Debug, PartialEq, Eq)]
pub struct AlreadyCreatedError;

impl Error for AlreadyCreatedError {}

impl Display for AlreadyCreatedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("shutdown handler already created")
    }
}

static CREATED: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub struct Shutdown {
    pub sender: mpsc::Sender<()>,
    receiver: mpsc::Receiver<()>,
}

#[derive(Debug, Clone)]
pub struct Agent {
    sender: mpsc::Sender<()>,
}

impl Agent {
    pub async fn start(&self) {
        println!("Shutdown requested");
        self.sender.send(()).await.ok();
    }
}

impl Shutdown {
    pub fn new() -> Result<Self, AlreadyCreatedError> {
        if (CREATED).swap(true, Ordering::SeqCst) {
            return Err(AlreadyCreatedError);
        }

        let (tx, rx) = mpsc::channel(1);
        let handle = register_handlers();

        let tx_for_handle = tx.clone();
        tokio::spawn(async move {
            handle.await;
            tx_for_handle.send(()).await.ok();
        });

        Ok(Self {
            sender: tx,
            receiver: rx,
        })
    }

    pub fn handle(&mut self) -> impl Future<Output = ()> + '_ {
        let rx = self.receiver.recv();

        async move {
            rx.await;
        }
    }

    pub fn extension(&self) -> Extension<Agent> {
        Extension(Agent {
            sender: self.sender.clone(),
        })
    }
}

fn register_handlers() -> impl Future<Output = ()> {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    async {
        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        println!("Received shutdown signal");
    }
}
