use crate::routes;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    info(title = "Cog", version = "0.1.0"),
    paths(routes::system::root, routes::system::health_check),
    components(schemas(RootResponse, HealthCheck, HealthCheckSetup))
)]
pub struct ApiDoc;

pub fn routes() -> SwaggerUi {
    SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi())
}

///////////////////////////////////////////////////////////////////////////////
///                                    /                                    ///
//////////////////////////////////////////////////////////////////////////////

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct RootResponse {
    /// Relative URL to Swagger UI
    pub docs_url: String,
    /// Relative URL to OpenAPI specification
    pub openapi_url: String,
}

///////////////////////////////////////////////////////////////////////////////
///                              /health-check                              ///
//////////////////////////////////////////////////////////////////////////////

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Health {
    Unknown,
    Starting,
    Ready,
    Busy,
    SetupFailed,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct HealthCheckSetup {
    /// Setup logs
    pub logs: String,
    /// Setup status
    pub status: String,
    /// Setup started time
    pub started_at: String,
    /// Setup completed time
    pub completed_at: String,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct HealthCheck {
    /// Current health status
    pub status: Health,
    /// Setup information
    pub setup: HealthCheckSetup,
}
