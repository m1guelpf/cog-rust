use axum::Router;

pub mod predict;
pub mod system;

use crate::schema;

pub fn handler() -> Router {
    Router::new()
        .merge(schema::routes())
        .merge(system::handler())
        .merge(predict::handler())
}
