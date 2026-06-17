#[cfg(target_arch = "x86_64")]
pub mod jit;

#[cfg(target_arch = "x86_64")]
pub use jit::CraneliftJIT;

#[cfg(not(target_arch = "x86_64"))]
pub struct CraneliftJIT;

#[cfg(not(target_arch = "x86_64"))]
impl CraneliftJIT {
    pub fn new() -> Self { CraneliftJIT }
    pub fn with_symbols<F: FnOnce(&mut ())>(_register: F) -> Self { CraneliftJIT }
    pub fn compile_function(&mut self, _name: &str, _body: usize, _num_params: usize, _num_locals: usize, _module: &arabi_compiler::bytecode::BytecodeModule) -> Option<*const u8> { None }
    pub fn compile_loop_function(&mut self, _name: &str, _body: usize, _num_params: usize, _num_locals: usize, _module: &arabi_compiler::bytecode::BytecodeModule) -> Option<*const u8> { None }
    pub fn is_compiled(&self, _body: usize) -> bool { false }
    pub fn is_loop_compiled(&self, _body: usize) -> bool { false }
    pub fn get_entry(&self, _body: usize) -> Option<*const u8> { None }
    pub fn get_loop_entry(&self, _body: usize) -> Option<*const u8> { None }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn register_runtime_symbols(_builder: &mut ()) {}

pub struct JitSymbol {
    pub name: &'static str,
    pub addr: *const u8,
}
