use aide::openapi::{MediaType, OpenApi, ReferenceOr, Response, SchemaObject, StatusCode};
use axum::http::Method;
use schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};

pub fn schema_with_properties<T: JsonSchema>(
	generator: &mut SchemaGenerator,
	cb: impl Fn(String, &mut schemars::schema::SchemaObject, usize),
) -> Schema {
	let mut schema = generator.root_schema_for::<T>().schema;
	let metadata = schema.metadata();

	metadata.title = Some(metadata.title.as_ref().map_or_else(
		|| T::schema_name(),
		|title| title.split('_').next().unwrap().to_string(),
	));

	let object = schema.object();
	for (index, (name, property)) in object.properties.clone().into_iter().enumerate() {
		let mut property: schemars::schema::SchemaObject = property.clone().into_object();

		cb(name.clone(), &mut property, index);
		object.properties.insert(name, property.into());
	}

	schemars::schema::Schema::Object(schema)
}

pub fn replace_request_schema(
	api: &mut OpenApi,
	path: &str,
	(method, media_type): (Method, &str),
	schema: schemars::schema::SchemaObject,
) -> Option<()> {
	let paths = api.paths.as_mut()?;
	let item = paths.paths.get_mut(path)?.as_item_mut()?;
	let operation = match method {
		Method::GET => item.get.as_mut()?,
		Method::PUT => item.put.as_mut()?,
		Method::POST => item.post.as_mut()?,
		Method::HEAD => item.head.as_mut()?,
		Method::TRACE => item.trace.as_mut()?,
		Method::DELETE => item.delete.as_mut()?,
		Method::OPTIONS => item.options.as_mut()?,
		_ => return None,
	};

	let body = operation.request_body.as_mut()?.as_item_mut()?;

	body.content.get_mut(media_type)?.schema = Some(SchemaObject {
		example: None,
		external_docs: None,
		json_schema: Schema::Object(schema),
	});

	Some(())
}

pub fn replace_response_schema(
	api: &mut OpenApi,
	path: &str,
	(method, status_code, media_type): (Method, StatusCode, String),
	json_schema: schemars::schema::SchemaObject,
) -> Option<()> {
	let paths = api.paths.as_mut()?;
	let item = paths.paths.get_mut(path)?.as_item_mut()?;
	let operation = match method {
		Method::GET => item.get.as_mut()?,
		Method::PUT => item.put.as_mut()?,
		Method::POST => item.post.as_mut()?,
		Method::HEAD => item.head.as_mut()?,
		Method::TRACE => item.trace.as_mut()?,
		Method::DELETE => item.delete.as_mut()?,
		Method::OPTIONS => item.options.as_mut()?,
		_ => return None,
	};

	let mut responses = operation.responses.clone().unwrap_or_default();
	let response = responses.responses.get(&status_code).cloned();
	let mut response = response.unwrap_or_else(|| ReferenceOr::Item(Response::default()));
	let response = response.as_item_mut()?;

	response.content.insert(
		media_type,
		MediaType {
			schema: Some(SchemaObject {
				example: None,
				external_docs: None,
				json_schema: Schema::Object(json_schema),
			}),
			..Default::default()
		},
	);

	responses
		.responses
		.insert(status_code, ReferenceOr::Item(response.clone()));
	operation.responses = Some(responses);

	Some(())
}
