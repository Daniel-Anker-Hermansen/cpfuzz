#[derive(clap::Parser, clap::ValueEnum, Clone, Copy, Debug)]
pub enum Language {
	Rust,
	RustDebug,
	Cpp,
}

#[derive(clap::Parser, Debug)]
pub struct Args {
	pub language: Language,
	pub name: String,
	pub specification: String,

	#[arg(short, long)]
	pub generate: bool,

	#[arg(short, long, value_name = "INTERACTOR")]
	pub interactive: Option<String>,

	#[arg(short, long, value_name = "COMPARATOR", conflicts_with("interactive"))]
	pub compare: Option<String>,

	#[arg(short, long, value_name = "VERIFYER", conflicts_with("interactive"), conflicts_with("compare"))]
	pub verify: Option<String>,
}

