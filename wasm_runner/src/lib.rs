mod compiler;
mod limitation_injector;
mod wasm_vm;
pub use wasm_vm::*;

pub(crate) type Error = Box<dyn std::error::Error>;
