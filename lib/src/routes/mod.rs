use aide::{
    axum::ApiRouter,
    openapi::{Info, OpenApi, StatusCode},
};
use axum::{http::Method, Extension};
use schemars::gen::{SchemaGenerator, SchemaSettings};

use crate::{
    helpers::{replace_request_schema, replace_response_schema},
    Cog,
};

use self::predict::{Prediction, PredictionRequest};

mod docs;
mod predict;
mod system;

pub fn handler<T: Cog>() -> axum::Router {
    let mut openapi = OpenApi {
        info: Info {
            title: "Cog".to_string(),
            version: "0.1.0".to_string(),
            ..Info::default()
        },
        ..OpenApi::default()
    };

    let router = ApiRouter::new()
        .merge(system::handler())
        .merge(predict::handler())
        .merge(docs::handler());

    router
        .finish_api(&mut openapi)
        .layer(Extension(tweak_openapi::<T>(openapi)))
}

fn tweak_openapi<T: Cog>(mut api: OpenApi) -> OpenApi {
    let mut generator = SchemaGenerator::new(SchemaSettings::draft07().with(|s| {
        s.inline_subschemas = true;
    }));

    replace_request_schema(
        &mut api,
        "/predictions",
        (Method::POST, "application/json"),
        generator.subschema_for::<PredictionRequest<T::Request>>(),
    )
    .unwrap();
    replace_response_schema(
        &mut api,
        "/predictions",
        (Method::POST, StatusCode::Code(200), "application/json"),
        generator.subschema_for::<Prediction<T::Response>>(),
    );

    api
}
