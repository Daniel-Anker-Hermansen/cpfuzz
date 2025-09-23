use std::{
	io::{self, Read, Write},
	process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use clap::Parser as _;

mod args;
mod error;
mod generator;
mod generator_bindings;

use args::Language;
use error::Error;

trait IoResultExt {
	fn ignore_broken_pipe(self) -> Self;
}

impl IoResultExt for io::Result<()> {
	fn ignore_broken_pipe(self) -> Self {
		match self {
			Ok(()) => Ok(()),
			Err(e) => match e.kind() {
				io::ErrorKind::BrokenPipe => Ok(()),
				_ => Err(e),
			},
		}
	}
}

fn transfer(mut read: impl Read, mut write: impl Write) -> io::Result<()> {
	let mut buf = [0; 512];
	while let n = read.read(&mut buf)?
		&& n > 0
	{
		write.write_all(&buf[..n]).ignore_broken_pipe()?;
	}
	Ok(())
}

impl Language {
	fn build(self, problem: &str) -> io::Result<()> {
		let (cmd, args): (&str, &[&str]) = match self {
			Language::Rust => ("cargo", &["build", "--bin", problem, "--release"]),
			Language::RustDebug => ("cargo", &["build", "--bin", problem]),
			Language::Cpp => ("g++", &["-O2", &format!("{problem}.cpp"), "-o", problem]),
			Language::CppSanitize => (
				"g++",
				&[
					"-g",
					"-fsanitize=address,undefined",
					&format!("{problem}.cpp"),
					"-o",
					problem,
				],
			),
		};
		let exit_code = Command::new(cmd).args(args).spawn()?.wait()?;
		if !exit_code.success() {
			std::process::exit(exit_code.code().unwrap_or(1));
		}
		Ok(())
	}

	fn spawn(self, problem: &str) -> io::Result<Child> {
		let path = match self {
			Language::Rust => &format!("target/release/{problem}"),
			Language::RustDebug => &format!("target/debug/{problem}"),
			Language::Cpp => &format!("./{problem}"),
			Language::CppSanitize => &format!("./{problem}"),
		};
		Command::new(path)
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::null())
			.spawn()
	}

	fn run(self, problem: &str, input: &[u8]) -> io::Result<(bool, String)> {
		let mut child = self.spawn(problem)?;
		child
			.stdin
			.as_mut()
			.expect("is piped")
			.write_all(input)
			.ignore_broken_pipe()?;
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
		let mut child = self.spawn(problem)?;
		let stdin = child.stdin.take().expect("is piped");
		let stdout = child.stdout.take().expect("is piped");
		Ok((stdin, stdout, child))
	}

	fn run_interacter(
		self,
		problem: &str,
		input: &[u8],
		child_stdin: ChildStdin,
		child_stdout: ChildStdout,
		mut interactee: Child,
	) -> io::Result<bool> {
		let mut child = self.spawn(problem)?;
		let mut stdin = child.stdin.take().expect("is piped");
		let stdout = child.stdout.take().expect("is piped");
		stdin.write_all(input).ignore_broken_pipe()?;
		let child_in = std::thread::spawn(|| transfer(stdout, child_stdin));
		let child_out = std::thread::spawn(|| transfer(child_stdout, stdin));
		child_in.join().expect("does not panic")?;
		child_out.join().expect("does not panic")?;
		let exit_code = child.wait()?;
		let interactee_exit_code = interactee.wait()?;
		Ok(exit_code.success() && interactee_exit_code.success())
	}
}

enum Status {
	Ok,
	Failed,
	PrimaryFailed,
	SecondaryFailed,
	VerifierFailed,
	DifferentOutputs,
}

impl Status {
	fn report(&self) {
		let message = match self {
			Status::Ok => "",
			Status::Failed => "\nExited with non-zero exit code",
			Status::PrimaryFailed => "\nPrimary exited with non-zero exit code",
			Status::SecondaryFailed => "\nSecondery exited with non-zero exit code",
			Status::VerifierFailed => "\nVerifier rejected the output",
			Status::DifferentOutputs => "\nDifferent outputs",
		};
		eprint!("{message}");
	}

	fn failed(&self) -> bool {
		!matches!(self, Status::Ok)
	}
}

enum Runner {
	Single { problem: String },
	Compare { primary: String, secondary: String },
	Interactive { problem: String, interactor: String },
	Verify { problem: String, verifier: String },
}

impl Runner {
	fn new(args: &args::Args) -> Result<Runner, Error> {
		args.language.build(&args.name)?;
		// Dear Bærbak, this if else switch is so beautiful, and nothing you ever have said
		// or will ever say will convince me otherwise.
		Ok(if let Some(interactor) = &args.interactive {
			args.language.build(interactor)?;
			Runner::Interactive {
				problem: args.name.clone(),
				interactor: interactor.clone(),
			}
		} else if let Some(verifier) = &args.verify {
			args.language.build(verifier)?;
			Runner::Verify {
				problem: args.name.clone(),
				verifier: verifier.clone(),
			}
		} else if let Some(comparator) = &args.compare {
			args.language.build(comparator)?;
			Runner::Compare {
				primary: args.name.clone(),
				secondary: comparator.clone(),
			}
		} else {
			Runner::Single {
				problem: args.name.clone(),
			}
		})
	}

	fn run(&self, languge: &Language, stdin: &[u8]) -> Result<Status, Error> {
		match self {
			Runner::Single { problem } => {
				let (status, _) = languge.run(problem, stdin)?;
				Ok(if status { Status::Ok } else { Status::Failed })
			}
			Runner::Compare { primary, secondary } => {
				let (primary_status, primary_out) = languge.run(primary, stdin)?;
				let (secondary_status, secondary_out) = languge.run(secondary, stdin)?;
				let stdout_ne = primary_out
					.split_whitespace()
					.ne(secondary_out.split_whitespace());
				Ok(if !primary_status {
					Status::PrimaryFailed
				} else if !secondary_status {
					Status::SecondaryFailed
				} else if stdout_ne {
					Status::DifferentOutputs
				} else {
					Status::Ok
				})
			}
			Runner::Interactive {
				problem,
				interactor,
			} => {
				let (chid_stdin, child_stdout, process) = languge.run_interactee(problem)?;
				let status =
					languge.run_interacter(interactor, stdin, chid_stdin, child_stdout, process)?;
				Ok(if status { Status::Ok } else { Status::Failed })
			}
			Runner::Verify { problem, verifier } => {
				let (problem_status, stdout) = languge.run(problem, stdin)?;
				if !problem_status {
					Ok(Status::Failed)
				} else {
					let mut new_stdin = stdin.to_vec();
					new_stdin.push(b'\n');
					new_stdin.extend_from_slice(stdout.as_bytes());
					let (status, _) = languge.run(verifier, &new_stdin)?;
					Ok(if status {
						Status::Ok
					} else {
						Status::VerifierFailed
					})
				}
			}
		}
	}
}

fn main() -> Result<(), Error> {
	let args = args::Args::parse();
	args.language.build(&args.name)?;
	let generator = generator::Generator::new(&args)?;
	let runner = Runner::new(&args)?;
	for _ in 1u64.. {
		eprint!(".");
		std::io::stderr().flush()?;
		let stdin = generator.generate()?;
		let result = runner.run(&args.language, &stdin)?;
		if result.failed() {
			result.report();
			eprintln!();
			std::io::stderr().write_all(&stdin).ignore_broken_pipe()?;
			eprintln!();
			std::fs::write("fuzz.in", &stdin)?;
			return Ok(());
		}
	}
	Ok(())
}
