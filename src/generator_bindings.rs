use std::{
	alloc::{self, Layout},
	io::Write as _,
};

struct Allocation {
	ptr: *mut u8,
	layout: Layout,
}

impl Allocation {
	fn new<T>(length: usize) -> Allocation {
		unsafe {
			let layout = Layout::array::<T>(length).unwrap();
			let ptr = alloc::alloc(layout);
			Allocation { ptr, layout }
		}
	}

	unsafe fn write<T>(&mut self, index: usize, value: T) {
		unsafe {
			self.ptr.cast::<T>().add(index).write(value);
		}
	}

	fn as_ptr<T>(&mut self) -> *const T {
		self.ptr.cast()
	}
}

impl Drop for Allocation {
	fn drop(&mut self) {
		unsafe {
			alloc::dealloc(self.ptr, self.layout);
		}
	}
}

pub struct ContextState {
	stdin: Vec<u8>,
	allocations: Vec<Allocation>,
}

impl ContextState {
	pub fn new() -> ContextState {
		ContextState {
			stdin: Vec::new(),
			allocations: Vec::new(),
		}
	}

	fn new_line(&mut self) {
		let _ = writeln!(&mut self.stdin);
	}

	fn i64(&mut self, lower: i64, higher: i64) -> i64 {
		let res = fastrand::i64(lower..=higher);
		let _ = write!(&mut self.stdin, "{} ", res);
		res
	}

	fn i64_array(&mut self, length: usize, lower: i64, higher: i64) -> *const i64 {
		let mut allocation = Allocation::new::<i64>(length);
		for i in 0..length {
			let res = fastrand::i64(lower..=higher);
			let _ = write!(&mut self.stdin, "{} ", res);
			unsafe {
				allocation.write(i, res);
			}
		}
		let ret = allocation.as_ptr();
		self.allocations.push(allocation);
		ret
	}

	fn ascii(&mut self, ascii: *const u8) {
		for i in 0.. {
			let res = unsafe { ascii.add(i).read() };
			if res == 0 {
				break;
			}
			self.stdin.push(res);
		}
	}

	pub fn into_stdin(self) -> Vec<u8> {
		self.stdin
	}
}

#[repr(C)]
pub struct Context<'ctx> {
	gen_new_line: extern "C" fn(&mut ContextState),
	gen_i64: extern "C" fn(&mut ContextState, i64, i64) -> i64,
	gen_i64_array: extern "C" fn(&mut ContextState, usize, i64, i64) -> *const i64,
	gen_ascii: extern "C" fn(&mut ContextState, *const u8),
	context_state: &'ctx mut ContextState,
}

impl<'ctx> Context<'ctx> {
	pub fn new(context_state: &'ctx mut ContextState) -> Context<'ctx> {
		Context {
			gen_new_line,
			gen_i64,
			gen_i64_array,
			gen_ascii,
			context_state,
		}
	}
}

extern "C" fn gen_new_line(context_state: &mut ContextState) {
	context_state.new_line();
}

extern "C" fn gen_i64(context_state: &mut ContextState, lower: i64, higher: i64) -> i64 {
	context_state.i64(lower, higher)
}

extern "C" fn gen_i64_array(
	context_state: &mut ContextState,
	length: usize,
	lower: i64,
	higher: i64,
) -> *const i64 {
	context_state.i64_array(length, lower, higher)
}

extern "C" fn gen_ascii(context_state: &mut ContextState, ascii: *const u8) {
	context_state.ascii(ascii);

}
