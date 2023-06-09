use aide::{
	axum::ApiRouter,
	openapi::{Components, Info, OpenApi, SchemaObject, StatusCode},
};
use axum::{http::Method, Extension};
use indexmap::indexmap;
use schemars::{
	gen::{SchemaGenerator, SchemaSettings},
	schema::SchemaObject as Schema,
};
use titlecase::titlecase;

use crate::{
	helpers::{replace_request_schema, replace_response_schema, schema_with_properties},
	prediction::{Request as PredictionRequest, Response as PredictionResponse},
	Cog,
};

mod docs;
mod predict;
mod system;

pub fn handler<T: Cog>() -> axum::Router {
	let mut generator = SchemaGenerator::new(SchemaSettings::openapi3().with(|settings| {
		settings.inline_subschemas = true;
	}));

	let mut openapi = OpenApi {
		info: Info {
			title: "Cog".to_string(),
			version: "0.1.0".to_string(),
			..Info::default()
		},
		components: Some(Components {
			schemas: indexmap! {
				"Input".to_string() => SchemaObject {
					example: None,
					external_docs: None,
					json_schema: schema_with_properties::<T::Request>(&mut generator, |name, schema, i| {
						schema.metadata().title = Some(titlecase(&name));
						schema.extensions.insert("x-order".to_string(), (i + 1).into());
					})
				},
				"PredictionRequest".to_string() => SchemaObject {
					example: None,
					external_docs: None,
					json_schema: schema_with_properties::<PredictionRequest>(&mut generator, |name, schema, _| {
						if name == "input" {
							schema.reference = Some("#/components/schemas/Input".to_string());
						}
					})
				},
				"Output".to_string() => SchemaObject {
					example: None,
					external_docs: None,
					json_schema: generator.subschema_for::<T::Response>()
				},
				"PredictionResponse".to_string() => SchemaObject {
					example: None,
					external_docs: None,
					json_schema: schema_with_properties::<PredictionResponse>(&mut generator, |name, schema, _| {
						if name == "input" {
							schema.reference = Some("#/components/schemas/Input".to_string());
						}

						if name == "output" {
							schema.reference = Some("#/components/schemas/Output".to_string());
						}
					})
				},
			},
			..Components::default()
		}),
		..OpenApi::default()
	};

	let router = ApiRouter::new()
		.merge(system::handler())
		.merge(predict::handler())
		.merge(docs::handler());

	router
		.finish_api(&mut openapi)
		.layer(Extension(tweak_openapi(openapi)))
}

fn tweak_openapi(mut api: OpenApi) -> OpenApi {
	replace_request_schema(
		&mut api,
		"/predictions",
		(Method::POST, "application/json"),
		Schema::new_ref("#/components/schemas/PredictionRequest".to_string()),
	)
	.unwrap();

	replace_response_schema(
		&mut api,
		"/predictions",
		(Method::POST, StatusCode::Code(200), "application/json"),
		Schema::new_ref("#/components/schemas/PredictionResponse".to_string()),
	);

	api
}
