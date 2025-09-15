use std::{
	collections::HashMap,
	io::{self, Read as _, Write},
	process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use clap::Parser as _;
use generator::{Context, ContextState};

mod generator;

#[derive(clap::Parser, clap::ValueEnum, Clone, Copy, Debug)]
enum Language {
	Rust,
	RustDebug,
	Cpp,
}

impl Language {
	fn build(self, problem: &str) -> io::Result<()> {
		let (cmd, args): (&str, &[&str]) = match self {
			Language::Rust => ("cargo", &["build", "--bin", problem, "--release"]),
			Language::RustDebug => ("cargo", &["build", "--bin", problem, "--release"]),
			Language::Cpp => ("g++", &["-O2", &format!("{problem}.cpp"), "-o", problem]),
		};
		let exit_code = Command::new(cmd).args(args).spawn()?.wait()?;
		if !exit_code.success() {
			std::process::exit(exit_code.code().unwrap_or(1));
		}
		Ok(())
	}

	fn run(self, problem: &str, input: &[u8]) -> io::Result<(bool, String)> {
		let path = match self {
			Language::Rust => &format!("target/release/{problem}"),
			Language::RustDebug => &format!("target/release/{problem}"),
			Language::Cpp => &format!("./{problem}"),
		};
		let mut child = Command::new(path)
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::null())
			.spawn()?;
		child.stdin.as_mut().expect("is piped").write_all(input)?;
		let mut stdout = Vec::new();
		child
			.stdout
			.as_mut()
			.expect("is piped")
			.read_to_end(&mut stdout)?;
		let exit_code = child.wait()?;
		Ok((
			exit_code.success(),
			String::from_utf8_lossy(&stdout).into_owned(),
		))
	}

	fn run_interactee(self, problem: &str) -> io::Result<(ChildStdin, ChildStdout, Child)> {
		let path = match self {
			Language::Rust => &format!("target/release/{problem}"),
			Language::RustDebug => &format!("target/release/{problem}"),
			Language::Cpp => &format!("./{problem}"),
		};
		let mut child = Command::new(path)
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::null())
			.spawn()?;
		let stdin = child.stdin.take().expect("is piped");
		let stdout = child.stdout.take().expect("is piped");
		Ok((stdin, stdout, child))
	}

	fn run_interacter(
		self,
		problem: &str,
		input: &[u8],
		mut child_stdin: ChildStdin,
		mut child_stdout: ChildStdout,
		mut interactee: Child,
	) -> io::Result<bool> {
		let path = match self {
			Language::Rust => &format!("target/release/{problem}"),
			Language::RustDebug => &format!("target/release/{problem}"),
			Language::Cpp => &format!("./{problem}"),
		};
		let mut child = Command::new(path)
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::null())
			.spawn()?;
		let mut stdin = child.stdin.take().expect("is piped");
		let mut stdout = child.stdout.take().expect("is piped");
		stdin.write_all(input)?;
		let child_in = std::thread::spawn(move || -> io::Result<()> {
			let mut buf = [0; 512];
			loop {
				let n = stdout.read(&mut buf)?;
				if n == 0 {
					break;
				}
				child_stdin.write_all(&buf[..n])?;
			}
			Ok(())
		});
		let child_out = std::thread::spawn(move || -> io::Result<()> {
			let mut buf = [0; 512];
			loop {
				let n = child_stdout.read(&mut buf)?;
				if n == 0 {
					break;
				}
				stdin.write_all(&buf[..n])?;
			}
			Ok(())
		});
		child_in.join().expect("does not panic")?;
		child_out.join().expect("does not panic")?;
		let exit_code = child.wait()?;
		let interactee_exit_code = interactee.wait()?;
		Ok(exit_code.success() && interactee_exit_code.success())
	}
}

enum Numeric {
	Integer(i64),
	Variable(String),
}

impl Numeric {
	fn evaluate(&self, store: &HashMap<&str, i64>) -> Result<i64, SpecificationError> {
		match self {
			Numeric::Integer(x) => Ok(*x),
			Numeric::Variable(x) => store
				.get(x.as_str())
				.copied()
				.ok_or(SpecificationError::Any),
		}
	}
}

enum SpecificationAtom {
	Integer {
		lower: Numeric,
		higher: Numeric,
		name: String,
	},
	Array {
		length: Numeric,
		lower: Numeric,
		higher: Numeric,
		_name: String,
	},
	Permuation {
		length: Numeric,
		_name: String,
	},
	NewLine,
}

struct Specification {
	atoms: Vec<SpecificationAtom>,
}

#[derive(Debug)]
enum SpecificationError {
	Any,
}

fn read_name<'a>(iter: &mut impl Iterator<Item = &'a str>) -> Result<String, SpecificationError> {
	iter.next()
		.ok_or(SpecificationError::Any)
		.map(str::to_string)
}

fn read_numeric<'a>(
	iter: &mut impl Iterator<Item = &'a str>,
) -> Result<Numeric, SpecificationError> {
	iter.next().ok_or(SpecificationError::Any).map(|s| {
		s.parse()
			.map(Numeric::Integer)
			.unwrap_or(Numeric::Variable(s.to_string()))
	})
}

impl Specification {
	fn parse(src: &str) -> Result<Specification, SpecificationError> {
		Ok(Specification {
			atoms: src.lines().try_fold(Vec::new(), |mut acc, line| {
				let mut tokens = line.split_ascii_whitespace();
				if !acc.is_empty() {
					acc.push(SpecificationAtom::NewLine);
				}
				while let Some(ty) = tokens.next() {
					match ty {
						"int" => {
							let name = read_name(&mut tokens)?;
							let lower = read_numeric(&mut tokens)?;
							let higher = read_numeric(&mut tokens)?;
							acc.push(SpecificationAtom::Integer {
								lower,
								higher,
								name,
							});
						}
						"arr" => {
							let name = read_name(&mut tokens)?;
							let length = read_numeric(&mut tokens)?;
							let lower = read_numeric(&mut tokens)?;
							let higher = read_numeric(&mut tokens)?;
							acc.push(SpecificationAtom::Array {
								length,
								lower,
								higher,
								_name: name,
							});
						}
						"perm" => {
							let name = read_name(&mut tokens)?;
							let length = read_numeric(&mut tokens)?;
							acc.push(SpecificationAtom::Permuation {
								length,
								_name: name,
							});
						}
						_ => return Err(SpecificationError::Any),
					}
				}
				Ok(acc)
			})?,
		})
	}

	fn generate(&self) -> Result<Vec<u8>, SpecificationError> {
		let mut store = HashMap::new();
		let mut stdin = Vec::new();
		for atom in &self.atoms {
			match atom {
				SpecificationAtom::Integer {
					lower,
					higher,
					name,
				} => {
					let lower = lower.evaluate(&store)?;
					let higher = higher.evaluate(&store)?;
					if higher < lower {
						return Err(SpecificationError::Any);
					}
					let val = fastrand::i64(lower..=higher);
					store.insert(name, val);
					write!(&mut stdin, "{val} ").expect("write to memory");
				}
				SpecificationAtom::Array {
					length,
					lower,
					higher,
					..
				} => {
					let length = length.evaluate(&store)?;
					if length < 0 {
						return Err(SpecificationError::Any);
					}
					let lower = lower.evaluate(&store)?;
					let higher = higher.evaluate(&store)?;
					for _ in 0..length {
						let val = fastrand::i64(lower..=higher);
						write!(&mut stdin, "{val} ").expect("write to memory");
					}
				}
				SpecificationAtom::Permuation { length, .. } => {
					let length = length.evaluate(&store)?;
					if length < 0 {
						return Err(SpecificationError::Any);
					}
					let mut perm: Vec<i64> = (1..=length).collect();
					fastrand::shuffle(&mut perm);
					for val in perm {
						write!(&mut stdin, "{val} ").expect("write to memory");
					}
				}
				SpecificationAtom::NewLine => stdin.push(b'\n'),
			}
		}
		Ok(stdin)
	}
}

#[derive(clap::Parser, Debug)]
struct Args {
	language: Language,
	name: String,
	specification: String,

	#[arg(short, long)]
	generate: bool,

	#[arg(short, long, value_name = "INTERACTOR")]
	interactive: Option<String>,

	#[arg(short, long, value_name = "COMPARATOR", conflicts_with("interactive"))]
	compare: Option<String>,
}

#[derive(Debug)]
enum Error {
	#[allow(dead_code)] // This is a false positive as it is read in the glue code of main.
	Io(io::Error),
	Specification(SpecificationError),
}

impl From<io::Error> for Error {
	fn from(value: io::Error) -> Self {
		Error::Io(value)
	}
}

impl From<SpecificationError> for Error {
	fn from(value: SpecificationError) -> Self {
		Error::Specification(value)
	}
}

enum Generator {
	Specification(Specification),
	Library {
		#[allow(dead_code)] // must be kept alive for function pointer to be safe.
		library: libloading::Library,
		generator: unsafe fn(&mut Context),
	},
}

impl Generator {
	fn new(args: &Args) -> Result<Generator, Error> {
		if args.generate {
			unsafe {
				let mut gcc = Command::new("g++")
					.args([
						&format!("{}.cpp", args.specification),
						"-x",
						"c",
						"-shared",
						"-o",
						"__cpfuzz_gen.so",
						"-",
					])
					.stdin(Stdio::piped())
					.spawn()?;
				write!(
					&mut gcc.stdin.as_mut().unwrap(),
					"{}",
					include_str!("cpfuzz.c")
				)?;
				let exit_code = gcc.wait()?;
				if !exit_code.success() {
					std::process::exit(exit_code.code().unwrap_or(1));
				}
				let library = libloading::Library::new("./__cpfuzz_gen.so").unwrap();
				let generator: unsafe fn(&mut Context) = std::mem::transmute(
					library
						.get::<unsafe fn(&Context)>(b"generate\0")
						.unwrap()
						.into_raw()
						.into_raw(),
				);
				Ok(Generator::Library { library, generator })
			}
		} else {
			let src = std::fs::read_to_string(&args.specification)?;
			Ok(Generator::Specification(Specification::parse(&src)?))
		}
	}

	fn generate(&self) -> Result<Vec<u8>, Error> {
		match self {
			Generator::Specification(specification) => specification.generate().map_err(Into::into),
			Generator::Library { generator, .. } => {
				let mut state = ContextState::new();
				let mut context = Context::new(&mut state);
				unsafe {
					generator(&mut context);
				}
				Ok(state.into_stdin())
			}
		}
	}
}

impl Drop for Generator {
	fn drop(&mut self) {
		if matches!(self, Generator::Library { .. }) {
			let _ = std::fs::remove_file("./__cpfuzz_gen.so");
		}
	}
}

fn main() -> Result<(), Error> {
	let args = Args::parse();
	args.language.build(&args.name)?;
	let generator = Generator::new(&args)?;
	if let Some(interactor) = args.interactive.as_ref() {
		args.language.build(interactor)?;
	}
	if let Some(name) = args.compare.as_ref() {
		args.language.build(name)?;
	}
	for _ in 1u64.. {
		eprint!(".");
		std::io::stderr().flush()?;
		let stdin = generator.generate()?;
		let result = if let Some(interactor) = args.interactive.as_ref() {
			let (child_stdin, child_stdout, child) = args.language.run_interactee(&args.name)?;
			args.language
				.run_interacter(interactor, &stdin, child_stdin, child_stdout, child)?
		} else {
			let (status, stdout) = args.language.run(&args.name, &stdin)?;
			if let Some(name) = args.compare.as_ref() {
				let (compare_status, compare_stdout) = args.language.run(name, &stdin)?;
				let stdout_eq = stdout.split_whitespace().eq(compare_stdout.split_whitespace());
				if !status {
					eprintln!();
					eprint!("Primary solver exited with non-zero code");
				}
				if !compare_status {
					eprintln!();
					eprint!("Secondary solver exited with non-zero code");
				}
				if status && compare_status && !stdout_eq {
					eprintln!();
					eprint!("Failed with different outputs");
				}
				status && compare_status && stdout_eq
			}
			else {
				status
			}
		};
		if !result {
			eprintln!();
			std::io::stderr().write_all(&stdin)?;
			eprintln!();
			std::fs::write("fuzz.in", &stdin)?;
			return Ok(());
		}
	}
	Ok(())
}
