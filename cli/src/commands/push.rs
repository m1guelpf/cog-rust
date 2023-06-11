use crate::{docker::Builder, Context};

pub fn handle(ctx: Context, image: Option<String>) {
	let builder = Builder::new(ctx.cwd);

	if builder.config.image.as_ref().or(image.as_ref()).is_none() {
		eprintln!("To push images, you must either set the 'image' option in the packages.metadata.cog of your Cargo.toml or pass an image name as an argument. For example, 'cargo cog push hotdog-detector'");
		std::process::exit(1);
	}

	let image_name = builder.build(image);

	Builder::push(&image_name);
	println!("Image '{image_name}' pushed");

	if image_name.starts_with("r8.im/") {
		println!(
			"Run your model on Replicate:\n    https://{}",
			image_name.replacen("r8.im", "replicate.com", 1)
		);
	}
}
