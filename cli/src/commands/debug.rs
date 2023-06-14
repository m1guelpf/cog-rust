use crate::Context;

pub fn handle(ctx: Context) {
	println!("{}", ctx.into_builder().generate_dockerfile());
}
