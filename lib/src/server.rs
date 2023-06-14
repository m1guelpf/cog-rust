use std::{env, net::SocketAddr};

use aide::openapi::{self, OpenApi};
use anyhow::Result;
use axum::{http::Method, Extension, Server};
use indexmap::indexmap;
use schemars::{
	gen::{SchemaGenerator, SchemaSettings},
	schema::SchemaObject as Schema,
};

use crate::{
	helpers::openapi::{replace_request_schema, replace_response_schema, schema_with_properties},
	prediction::{self, Prediction},
	routes,
	shutdown::Shutdown,
	Cli, Cog,
};

#[allow(clippy::redundant_pub_crate)]
pub(crate) async fn start<T: Cog + 'static>(args: Cli) -> Result<()> {
	if let Some(url) = args.upload_url {
		env::set_var("UPLOAD_URL", url.to_string());
	}

	let mut openapi = generate_schema::<T>();
	let router = routes::handler().finish_api(&mut openapi);
	tweak_generated_schema(&mut openapi);

	let shutdown = Shutdown::new(args.await_explicit_shutdown)?;
	if args.dump_schema_and_exit {
		println!("{}", serde_json::to_string(&openapi).unwrap());
		shutdown.start();
		return Ok(());
	}

	let prediction = Prediction::setup::<T>(shutdown.clone());

	let router = router
		.layer(Extension(openapi))
		.layer(shutdown.extension())
		.layer(prediction.extension());

	let addr = SocketAddr::from((
		[0, 0, 0, 0],
		env::var("PORT").map_or(Ok(5000), |p| p.parse())?,
	));

	tracing::info!("Starting server on {addr}...");
	Server::bind(&addr)
		.serve(router.into_make_service())
		.with_graceful_shutdown(shutdown.handle())
		.await?;

	Ok(())
}

fn generate_schema<T: Cog>() -> OpenApi {
	let mut generator = SchemaGenerator::new(SchemaSettings::openapi3().with(|settings| {
		settings.inline_subschemas = true;
	}));

	OpenApi {
		info: openapi::Info {
			title: "Cog".to_string(),
			version: "0.1.0".to_string(),
			..openapi::Info::default()
		},
		components: Some(openapi::Components {
			schemas: indexmap! {
				"Input".to_string() => openapi::SchemaObject {
					example: None,
					external_docs: None,
					json_schema: schema_with_properties::<T::Request>(&mut generator, |name, schema, i| {
						schema.metadata().title = Some(titlecase::titlecase(&name));
						schema.extensions.insert("x-order".to_string(), (i + 1).into());
					})
				},
				"PredictionRequest".to_string() => openapi::SchemaObject {
					example: None,
					external_docs: None,
					json_schema: schema_with_properties::<prediction::Request>(&mut generator, |name, schema, _| {
						if name == "input" {
							schema.reference = Some("#/components/schemas/Input".to_string());
						}
					})
				},
				"Output".to_string() => openapi::SchemaObject {
					example: None,
					external_docs: None,
					json_schema: generator.subschema_for::<T::Response>()
				},
				"PredictionResponse".to_string() => openapi::SchemaObject {
					example: None,
					external_docs: None,
					json_schema: schema_with_properties::<prediction::Response>(&mut generator, |name, schema, _| {
						if name == "input" {
							schema.reference = Some("#/components/schemas/Input".to_string());
						}

						if name == "output" {
							schema.reference = Some("#/components/schemas/Output".to_string());
						}
					})
				},
			},
			..openapi::Components::default()
		}),
		..OpenApi::default()
	}
}

fn tweak_generated_schema(openapi: &mut OpenApi) {
	replace_request_schema(
		openapi,
		"/predictions",
		(Method::POST, "application/json"),
		Schema::new_ref("#/components/schemas/PredictionRequest".to_string()),
	)
	.unwrap();

	replace_response_schema(
		openapi,
		"/predictions",
		(
			Method::POST,
			openapi::StatusCode::Code(200),
			"application/json",
		),
		Schema::new_ref("#/components/schemas/PredictionResponse".to_string()),
	);
}
