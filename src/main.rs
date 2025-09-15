use std::{
	io::{self, Read as _, Write},
	process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use clap::Parser as _;

mod args;
mod error;
mod generator;
mod generator_bindings;

use args::Language;
use error::Error;

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

fn main() -> Result<(), Error> {
	let args = args::Args::parse();
	args.language.build(&args.name)?;
	let generator = generator::Generator::new(&args)?;
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
				let stdout_eq = stdout
					.split_whitespace()
					.eq(compare_stdout.split_whitespace());
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
			} else {
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
