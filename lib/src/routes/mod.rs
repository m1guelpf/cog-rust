use aide::axum::ApiRouter;

mod docs;
mod predict;
mod system;

pub fn handler() -> ApiRouter {
	ApiRouter::new()
		.merge(system::handler())
		.merge(predict::handler())
		.merge(docs::handler())
}
