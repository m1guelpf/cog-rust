use std::path::Path;

use crate::Context;
use cargo_toml::Manifest;

pub fn handle(ctx: &Context) {
	let dockerfile =
		include_str!("../templates/Dockerfile").replace("{:bin_name}", &get_binary_name(&ctx.cwd));

	println!("{dockerfile}");
}

fn get_binary_name(path: &Path) -> String {
	let cargo_toml = Manifest::from_path(path.join("Cargo.toml"))
		.expect("Failed to read Cargo.toml. Make sure you are in the root of your Cog project.");

	path.join("src/main.rs").metadata().expect(
    "Couldn't find the project's entry point. Make sure you are in the root of your Cog project.",
    );

	cargo_toml
		.package
		.as_ref()
		.expect("Couldn't find the package section in Cargo.toml.")
		.name
		.clone()
}
