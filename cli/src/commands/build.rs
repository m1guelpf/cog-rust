use crate::{docker::Builder, Context};

pub async fn handle(ctx: Context, tag: Option<String>) {
	let builder = Builder::new(ctx.cwd);

	let image_name = builder.build(tag).await;
	println!("Image built as {image_name}");
}
