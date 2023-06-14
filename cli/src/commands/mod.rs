use clap::Subcommand;

use crate::Context;

mod build;
mod debug;
mod login;
mod predict;
mod push;

#[derive(Debug, Subcommand)]
pub enum Command {
	/// Log in to Replicate's Docker registry
	Login {
		/// Pass login token on stdin instead of opening a browser. You can find your Replicate login token at https://replicate.com/auth/token
		#[clap(long)]
		token_stdin: bool,
		/// Registry host
		#[clap(hide = true, default_value = "r8.im")]
		registry: String,
	},
	/// Generate a Dockerfile for your project
	#[clap(hide = true)]
	Debug,

	/// Build the model in the current directory into a Docker image
	Build {
		/// A name for the built image in the form 'repository:tag'
		#[clap(short, long)]
		tag: Option<String>,
	},

	/// Build and push model in current directory to a Docker registry
	Push {
		/// A name for the built image
		image: Option<String>,
	},

}

pub async fn exec(ctx: Context, command: Command) {
	match command {
		Command::Debug => debug::handle(ctx),
		Command::Build { tag } => build::handle(ctx, tag),
		Command::Push { image } => push::handle(ctx, &image),
		Command::Login {
			registry,
			token_stdin,
		} => login::handle(token_stdin, registry).await,
	};
}
