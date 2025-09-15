use std::io;

#[derive(Debug)]
pub enum SpecificationError {
	Any,
}

#[derive(Debug)]
pub enum Error {
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
