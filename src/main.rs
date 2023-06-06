#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use crate::shutdown::Shutdown;
use std::{env, net::SocketAddr};

mod errors;
mod routes;
mod schema;
mod shutdown;

#[tokio::main]
async fn main() {
    let mut shutdown = Shutdown::new().unwrap();

    let addr = SocketAddr::from((
        [0, 0, 0, 0],
        env::var("PORT").map_or(5000, |p| p.parse().unwrap()),
    ));
    println!("Listening on {addr}");

    let app = routes::handler().layer(shutdown.extension());

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown.handle())
        .await
        .unwrap();
}
