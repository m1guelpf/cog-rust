use std::process::Command;

#[derive(Debug)]
pub struct Dockerfile(String);

impl Dockerfile {
	pub const fn new() -> Self {
		Self(String::new())
	}

	pub fn run_multiple(mut self, command: &[&Command]) -> Self {
		self.0.push_str(&format!(
			"RUN {}\n",
			command
				.iter()
				.map(|cmd| format!(
					"{} {}",
					cmd.get_program().to_string_lossy(),
					cmd.get_args()
						.map(|arg| arg.to_string_lossy().to_string())
						.collect::<Vec<String>>()
						.join(" ")
				))
				.collect::<Vec<String>>()
				.join(" && ")
		));

		self
	}

	pub fn env(mut self, key: &str, value: &str) -> Self {
		self.0.push_str(&format!("ENV {key}={value}\n"));

		self
	}

	pub fn copy(mut self, src: &str, dest: &str) -> Self {
		self.0.push_str(&format!("COPY {src} {dest}\n"));

		self
	}
}

#[allow(clippy::module_name_repetitions)]
pub trait DockerfileExt {
	fn build(self) -> Self;
	fn for_bin(self, bin: &str) -> Self;
	fn handler(self, slot: &str, value: impl FnOnce() -> Option<Dockerfile>) -> Self;
}

impl DockerfileExt for String {
	fn for_bin(self, bin: &str) -> Self {
		self.replace("{:bin_name}", bin)
	}

	fn handler(self, slot: &str, value: impl FnOnce() -> Option<Dockerfile>) -> Self {
		let value = value();

		match value {
			Some(value) => self.replace(
				&format!("#SLOT {slot}"),
				&format!("{}#SLOT {slot}\n", value.0),
			),
			None => self,
		}
	}

	fn build(self) -> Self {
		self.lines()
			.filter(|line| !line.starts_with("#SLOT"))
			.collect::<Vec<&str>>()
			.join("\n")
			.replace("\n\n", "\n")
	}
}
