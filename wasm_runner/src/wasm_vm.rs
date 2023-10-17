use crate::{compiler::compile, Error};
use script_api::*;
use thiserror::Error;
use wasmer::{imports, Cranelift, Instance, MemoryView, Module, Store, Value, WasmPtr};

const INSTRUCTIONS_PER_TICK: i32 = 1_000_000;

pub struct WasmVM {
    store: wasmer::Store,
    memory: wasmer::Memory,
    script_output_pointer: WasmPtr<u8>,
    input_pointer: WasmPtr<u8>,
    panic_pointer: WasmPtr<u8>,
    run: wasmer::Function,
    reset_instructions: wasmer::Function,
    get_instructions: wasmer::Function,
    debug_text_pointer: WasmPtr<u8>,
    get_text_size: wasmer::Function,
    erase_text: wasmer::Function,
}

impl WasmVM {
    pub fn new(code: String) -> Result<Self, Error> {
        //Take the text code and compile it into a wasm module to be loaded
        let wasm_data = compile(code)?;
        let mut store = Store::new(Cranelift::new());
        let module = Module::new(&store, wasm_data)?;

        //Get the necessary variable pointers
        let import_object = imports! {};
        let instance = Instance::new(&mut store, &module, &import_object)?;

        let memory = instance.exports.get_memory("memory")?.clone();

        let action_offset: i32 = instance
            .exports
            .get_global("SCRIPT_OUTPUT_BUFFER")?
            .get(&mut store)
            .i32()
            .ok_or(VMError::VMInitErrorScriptOutput)?;
        let script_output_pointer: WasmPtr<u8> = WasmPtr::new(action_offset as u32);

        let input_offset: i32 = instance
            .exports
            .get_global("DATA_INPUT_BUFFER")?
            .get(&mut store)
            .i32()
            .ok_or(VMError::VMInitErrorInputBuffer)?;
        let input_pointer: WasmPtr<u8> = WasmPtr::new(input_offset as u32);

        let panic_buffer_offset: i32 = instance
            .exports
            .get_global("PANIC_BUFFER")?
            .get(&mut store)
            .i32()
            .ok_or(VMError::VMErrorNoPanicBuffer)?;
        let panic_pointer: WasmPtr<u8> = WasmPtr::new(panic_buffer_offset as u32);

        let debug_text_offset: i32 = instance
            .exports
            .get_global("TEXT_BUFFER")?
            .get(&mut store)
            .i32()
            .ok_or(VMError::VMErrorNoDebugBuffer)?;
        let debug_text_pointer: WasmPtr<u8> = WasmPtr::new(debug_text_offset as u32);

        //Get functions needed to run script
        let run = instance.exports.get_function("export_run")?.clone();
        let reset_instructions = instance.exports.get_function("reset_instructions")?.clone();
        let get_instructions = instance.exports.get_function("get_instructions")?.clone();
        let get_text_size = instance.exports.get_function("get_text_size")?.clone();
        let erase_text = instance.exports.get_function("erase_text")?.clone();

        Ok(Self {
            store,
            memory,
            script_output_pointer,
            input_pointer,
            panic_pointer,
            run,
            reset_instructions,
            get_instructions,
            debug_text_pointer,
            get_text_size,
            erase_text,
        })
    }

    ///Resets a script for another run
    fn reset_script(&mut self) -> Result<(), Error> {
        self.reset_instructions
            .call(&mut self.store, &[INSTRUCTIONS_PER_TICK.into()])?;
        self.erase_text.call(&mut self.store, &[])?;

        let memory_view = self.memory.view(&self.store);

        //Get the byte array to store outputs and reset it to nothing
        let output_slice = self
            .script_output_pointer
            .slice(&memory_view, MAX_OUTPUT_SIZE as u32)
            .expect("Read action slice");
        let empty_array = [0_u8; MAX_OUTPUT_SIZE];
        output_slice.write_slice(&empty_array)?;

        Ok(())
    }

    ///Replaces the data input buffer with new data
    fn set_input(&mut self, inputs: Inputs) -> Result<(), Error> {
        let memory_view = self.memory.view(&self.store);

        //Serialize hashmap, get size, have the size be the first 8 bytes
        let mut map_data = bincode::serialize(&inputs)?;
        let mut final_data: Vec<u8> = vec![];
        let mut map_size = (map_data.len() as u64).to_le_bytes().to_vec();
        final_data.append(&mut map_size);
        final_data.append(&mut map_data);
        final_data.resize(2048, 0);

        //Write devices
        let devices_index = self.input_pointer.slice(&memory_view, 2048)?;
        devices_index.write_slice(&mut final_data)?;
        Ok(())
    }

    ///Parse the action buffer the script modified to get the actions that want to be performed
    fn read_actions(&self) -> Result<Vec<ScriptAction>, Error> {
        let memory_view = self.memory.view(&self.store);

        //Extract byte array from memory
        let action_slice = self
            .script_output_pointer
            .slice(&memory_view, MAX_OUTPUT_SIZE as u32)?;

        let mut action_slice_buffer: Vec<u8> = vec![];
        action_slice_buffer.resize(MAX_OUTPUT_SIZE, 0);

        action_slice.read_slice(&mut action_slice_buffer)?;

        //Split array between the size u32 and the rest of the body
        let (size_bytes, body) = action_slice_buffer.split_at(4);

        //Get size
        let mut u32_buffer: [u8; 4] = [0; 4];
        u32_buffer.copy_from_slice(size_bytes);
        let size = u32::from_le_bytes(u32_buffer);

        //Find the actual content of buffer and parse
        let content: &[u8] = &body[0..size as usize];

        Ok(bincode::deserialize(content)?)
    }

    ///Gets the instructions variable from the module and subtracts INSTRUCTIONS_PER_TICK to figure out how much gas has been used
    pub fn get_instructions_used(&mut self) -> Result<i32, Error> {
        let res = self.get_instructions.call(&mut self.store, &[])?.to_vec();
        if let Some(Value::I32(num)) = res.get(0) {
            let gas_used = INSTRUCTIONS_PER_TICK - num;
            return Ok(gas_used);
        }

        Err(Box::new(VMError::VMErrorNoGas))
    }

    ///Reads the TEXT_BUFFER global to extract any text created by the debug!() macro
    pub fn read_debug_string(&mut self) -> Result<String, Error> {
        let res = self.get_text_size.call(&mut self.store, &[])?;
        let memory_view = self.memory.view(&self.store);

        if let Some(Value::I32(length)) = res.get(0) {
            let mut byte_buffer: Vec<u8> = vec![];
            byte_buffer.resize(*length as usize, 0);
            let slice = self
                .debug_text_pointer
                .slice(&memory_view, *length as u32)?;
            slice.read_slice(&mut byte_buffer)?;
            return Ok(String::from_utf8(byte_buffer)?);
        }
        Ok(String::default())
    }

    fn read_vec<T: Default + Clone>(
        &self,
        memory_view: &MemoryView,
        offset: u32,
        length: u32,
    ) -> Option<Vec<T>> {
        let ptr: WasmPtr<u8> = WasmPtr::new(offset);
        let byte_length = length.saturating_mul(std::mem::size_of::<T>() as u32);
        let slice = ptr.slice(memory_view, byte_length).ok()?;
        let byte_vec = slice.read_to_vec().ok()?;
        let src_ptr = unsafe { std::mem::transmute::<*const u8, *const T>(byte_vec.as_ptr()) };
        let src_slice = unsafe { std::slice::from_raw_parts(src_ptr, length as usize) };
        Some(src_slice.to_vec())
    }

    ///Pulls the panic buffer from the module, attempting so see if the script panicked
    fn get_panic_data(&mut self) -> String {
        let memory_view = self.memory.view(&self.store);
        if let Some(vec) = self.read_vec(
            &memory_view,
            self.panic_pointer.offset(),
            script_api::panic::PANIC_BUFFER_SIZE as u32,
        ) {
            let null_pos = vec.iter().position(|&x| x == 0).unwrap_or(vec.len());
            let msg = String::from_utf8_lossy(&vec[0..null_pos]).to_string();
            return msg;
        }
        return "".to_string();
    }

    ///Call the "export_run" function once  and then check to see if the VM ran out of instructions are panicked
    pub fn run_tick(
        &mut self,
        inputs: Inputs,
    ) -> Result<Vec<ScriptAction>, Error> {
        self.reset_script()?;

        self.set_input(inputs)?;

        if let Err(e) = self.run.call(&mut self.store, &[]) {
            //Check to see if VM ran out of instructions
            if let Ok(intructions_used) = self.get_instructions_used() {
                if intructions_used >= INSTRUCTIONS_PER_TICK - 10000 {
                    return Err(Box::new(VMError::VMProcLimitReached));
                }
            }

            //Check to see if some code in the VM panicked
            let panic_str = self.get_panic_data();
            if !panic_str.is_empty() {
                return Err(Box::new(VMError::VMPanic(panic_str)));
            }

            //Must be a runtime error then
            return Err(Box::new(e));
        }
        Ok(self.read_actions()?)
    }
}

#[derive(Error, Debug)]
pub enum VMError {
    #[error("Module is missing SCRIPT_OUTPUT_BUFFER global")]
    VMInitErrorScriptOutput,
    #[error("Module is missing DATA_INPUT_BUFFER global")]
    VMInitErrorInputBuffer,
    #[error("Module is missing TEXT_BUFFER global")]
    VMErrorNoDebugBuffer,
    #[error("Module is missing PANIC_BUFFER global")]
    VMErrorNoPanicBuffer,
    #[error("No gas variable found in module")]
    VMErrorNoGas,
    #[error("WASM VM panicked and output the following error")]
    VMPanic(String),
    #[error("WASM VM reached maximum amount of instructs allowed and crashed")]
    VMProcLimitReached,
    #[error("WASM VM failed to compile code for the following reason")]
    VMCompileFail(String),
}
