use std::io::Write as _;

pub struct ContextState {
	stdin: Vec<u8>,
}

impl ContextState {
	pub fn new() -> ContextState {
		ContextState { stdin: Vec::new() }
	}

	fn new_line(&mut self) {
		let _ = writeln!(&mut self.stdin);
	}

	fn i64(&mut self, val: i64) {
		let _ = write!(&mut self.stdin, "{val} ");
	}

	fn ascii(&mut self, ascii: *const u8) {
		for i in 0.. {
			let res = unsafe { ascii.add(i).read() };
			if res == 0 {
				break;
			}
			self.stdin.push(res);
			self.stdin.push(b' ');
		}
	}

	pub fn into_stdin(self) -> Vec<u8> {
		self.stdin
	}
}

#[repr(C)]
pub struct Context<'ctx> {
	write_nl: extern "C" fn(&mut ContextState),
	write_i64: extern "C" fn(&mut ContextState, i64),
	write_ascii: extern "C" fn(&mut ContextState, *const u8),
	rand_i64: extern "C" fn(i64, i64) -> i64,
	context_state: &'ctx mut ContextState,
}

impl<'ctx> Context<'ctx> {
	pub fn new(context_state: &'ctx mut ContextState) -> Context<'ctx> {
		Context {
			write_nl,
			write_i64,
			write_ascii,
			rand_i64,
			context_state,
		}
	}
}

extern "C" fn write_nl(context_state: &mut ContextState) {
	context_state.new_line();
}

extern "C" fn write_i64(context_state: &mut ContextState, val: i64) {
	context_state.i64(val);
}

extern "C" fn write_ascii(context_state: &mut ContextState, ascii: *const u8) {
	context_state.ascii(ascii);
}

extern "C" fn rand_i64(lower: i64, higher: i64) -> i64 {
	fastrand::i64(lower..=higher)
}
