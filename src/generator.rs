use std::{collections::HashMap, io::Write as _, process::{Command, Stdio}};

use crate::{args, error::{Error, SpecificationError}, generator_bindings::{Context, ContextState}};

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

pub struct Specification {
	atoms: Vec<SpecificationAtom>,
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

pub enum Generator {
	Specification(Specification),
	Library {
		#[allow(dead_code)] // must be kept alive for function pointer to be safe.
		library: libloading::Library,
		generator: unsafe fn(&mut Context),
	},
}

impl Generator {
	pub fn new(args: &args::Args) -> Result<Generator, Error> {
		if args.generate {
			unsafe {
				let mut gcc = Command::new("g++")
					.args([
						&format!("{}.cpp", args.specification),
						"-x",
						"c++",
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
					include_str!("cpfuzz.cpp")
				)?;
				let exit_code = gcc.wait()?;
				if !exit_code.success() {
					std::process::exit(exit_code.code().unwrap_or(1));
				}
				let library = libloading::Library::new("./__cpfuzz_gen.so").unwrap();
				let generator: unsafe fn(&mut Context) = std::mem::transmute(
					library
						.get::<unsafe fn(&Context)>(b"__generate\0")
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

	pub fn generate(&self) -> Result<Vec<u8>, Error> {
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
