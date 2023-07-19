use base64::{engine::general_purpose::STANDARD as Base64, Engine};
use dataurl::DataUrl;
use mime_guess::Mime;
use schemars::schema::SchemaObject;
use serde_json::Value;
use std::{collections::HashMap, fs, path::PathBuf, str::FromStr};

use crate::{
	docker::{Docker, Predictor},
	Context,
};

pub async fn handle(
	ctx: Context,
	image: Option<String>,
	inputs: Option<Vec<String>>,
	output: Option<PathBuf>,
) {
	let image = image.map_or_else(
		|| ctx.clone().into_builder().build(None),
		|image| {
			if Docker::inspect_image(&image).is_err() {
				Docker::pull(&image).unwrap();
			}

			image
		},
	);

	println!("Starting Docker image {image} and running setup()...");
	let mut predictor = Predictor::new(image);

	predictor.start().await;
	predict_individual_inputs(&mut predictor, inputs, output).await;
}

async fn predict_individual_inputs(
	predictor: &mut Predictor,
	inputs: Option<Vec<String>>,
	mut output: Option<PathBuf>,
) {
	println!("Running prediction...");
	let schema = predictor.get_schema().unwrap();
	let inputs = inputs
		.map(|inputs| parse_inputs(inputs, &schema))
		.unwrap_or_default();

	let prediction = predictor.predict(inputs).await.unwrap();

	let response_schema = (|| {
		schema
			.extensions
			.get("components")?
			.get("schemas")?
			.get("Output")
	})()
	.unwrap();

	let out = parse_response(&prediction.output.unwrap(), response_schema, &mut output);

	match out {
		SerializedResponse::Text(text) => println!("{text}"),
		SerializedResponse::Bytes(bytes) => {
			let output = output.expect("No output file specified");

			fs::write(&output, bytes).expect("Failed to write output file");
			println!("Written output to {}", output.display());
		},
	}
}

#[derive(Debug)]
enum SerializedResponse {
	Text(String),
	Bytes(Vec<u8>),
}

fn parse_response(
	prediction: &Value,
	schema: &Value,
	output: &mut Option<PathBuf>,
) -> SerializedResponse {
	if schema.get("type") == Some(&Value::String("array".to_string())) {
		todo!("array response not yet supported");
	}

	if schema.get("type") == Some(&Value::String("string".to_string()))
		&& schema.get("format") == Some(&Value::String("uri".to_string()))
	{
		let url = prediction.as_str().unwrap();

		if !url.starts_with("data:") {
			return SerializedResponse::Text(url.to_string());
		}

		let dataurl = DataUrl::parse(url).expect("Failed to parse data URI");

		if output.is_none() {
			*output = Some(PathBuf::from(format!(
				"output{}",
				mime_guess::get_mime_extensions(
					&Mime::from_str(dataurl.get_media_type())
						.unwrap_or(mime_guess::mime::APPLICATION_OCTET_STREAM),
				)
				.and_then(<[&str]>::last)
				.map(|e| format!(".{e}"))
				.unwrap_or_default()
			)));
		}

		return SerializedResponse::Bytes(dataurl.get_data().to_vec());
	}

	if schema.get("type") == Some(&Value::String("string".to_string())) {
		return SerializedResponse::Text(
			prediction
				.as_str()
				.expect("Expected prediction to be a string")
				.to_string(),
		);
	}

	SerializedResponse::Text(serde_json::to_string(&prediction).unwrap())
}

fn parse_inputs(inputs: Vec<String>, schema: &SchemaObject) -> HashMap<String, String> {
	let mut key_vals = HashMap::new();

	for input in inputs {
		let (name, mut value) = if input.contains('=') {
			let split: [String; 2] = input
				.splitn(2, '=')
				.map(str::to_string)
				.collect::<Vec<String>>()
				.try_into()
				.expect(
					"Failed to parse input. Please specify inputs in the format '-i name=value'",
				);

			(split[0].clone(), split[1].clone())
		} else {
			(get_first_input(schema).expect("Could not determine the default input based on the order of the inputs. Please specify inputs in the format '-i name=value'"), input)
		};

		if value.starts_with('"') && value.ends_with('"') {
			value = value[1..value.len() - 1].to_string();
		}

		key_vals.insert(
			name.clone(),
			value.strip_prefix('@').map_or_else(
				|| value.clone(),
				|path_str| {
					let bytes = fs::read(PathBuf::from(path_str))
						.expect("Couldn't find {path_str} file (for {name})");

					let mime = tree_magic_mini::from_u8(bytes.as_slice());

					format!("data:{mime};base64,{}", Base64.encode(bytes))
				},
			),
		);
	}

	key_vals
}

pub fn get_first_input(schema: &SchemaObject) -> Option<String> {
	let input_properties = schema
		.extensions
		.get("components")?
		.get("schemas")?
		.get("Input")?
		.get("properties")?;

	for (k, v) in input_properties.as_object()? {
		let Some(order) = v.get("x-order").and_then(|o| match o {
			Value::Number(n) => n.as_i64(),
			_ => None,
		}) else {
			continue;
		};

		if order == 0 {
			return Some(k.clone());
		}
	}

	None
}
