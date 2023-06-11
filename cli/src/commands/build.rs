use crate::{docker::Builder, Context};

pub fn handle(ctx: Context, tag: Option<String>) {
	let builder = Builder::new(ctx.cwd);

	let image_name = builder.build(tag);
	println!("Image built as {image_name}");
}
