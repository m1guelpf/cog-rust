use crate::Context;

pub fn handle(ctx: Context, tag: Option<String>) {
	let image_name = ctx.into_builder().build(tag);
	println!("Image built as {image_name}");
}
