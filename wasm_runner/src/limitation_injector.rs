// Taken from https://github.com/rlane/oort3/blob/master/shared/simulator/src/vm/limiter.rs
// I would write it myself but this is exactly what I would do anyway
use std::error::Error;
use walrus::{ir::*, FunctionBuilder, GlobalId, InitExpr, LocalFunction, ValType};

pub fn rewrite(wasm: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut module = walrus::Module::from_buffer(wasm)?;

    let instruction_global =
        module
            .globals
            .add_local(ValType::I32, true, InitExpr::Value(Value::I32(0)));

    // Rewrite each block to check and decrement instrucions
    for (_, func) in module.funcs.iter_local_mut() {
        rewrite_function(func, instruction_global);
    }

    // Create a reset_instruction function to reset instruction limit
    {
        let mut func = FunctionBuilder::new(&mut module.types, &[ValType::I32], &[]);
        let amount = module.locals.add(ValType::I32);
        func.func_body()
            .local_get(amount)
            .global_set(instruction_global);
        let reset_gas = func.finish(vec![amount], &mut module.funcs);
        module.exports.add("reset_instructions", reset_gas);
    }

    // Create a get_instruction function to allow the VM to check the instructions
    {
        let mut func = FunctionBuilder::new(&mut module.types, &[], &[ValType::I32]);
        func.func_body().global_get(instruction_global);
        let get_gas = func.finish(vec![], &mut module.funcs);
        module.exports.add("get_instructions", get_gas);
    }

    Ok(module.emit_wasm())
}

fn rewrite_function(func: &mut LocalFunction, gas_global: GlobalId) {
    let block_ids: Vec<_> = func.blocks().map(|(block_id, _block)| block_id).collect();
    for block_id in block_ids {
        rewrite_block(func, block_id, gas_global);
    }
}

/// Number of injected metering instructions (needed to calculate final instruction size).
const METERING_INSTRUCTION_COUNT: usize = 8;

fn rewrite_block(func: &mut LocalFunction, block_id: InstrSeqId, gas_global: GlobalId) {
    let block = func.block_mut(block_id);
    let block_instrs = &mut block.instrs;
    let block_len = block_instrs.len();
    let block_cost = block_len as i32;

    let builder = func.builder_mut();
    let mut builder = builder.dangling_instr_seq(None);
    let seq = builder
        // if unsigned(globals[instruction]) < unsigned(block_cost) { throw(); }
        .global_get(gas_global)
        .i32_const(block_cost)
        .binop(BinaryOp::I32LtU)
        .if_else(
            None,
            |then| {
                then.unreachable();
            },
            |_else| {},
        )
        // globals[instruction] -= block_cost;
        .global_get(gas_global)
        .i32_const(block_cost)
        .binop(BinaryOp::I32Sub)
        .global_set(gas_global);

    let mut new_instrs = Vec::with_capacity(block_len + METERING_INSTRUCTION_COUNT);
    new_instrs.append(seq.instrs_mut());

    let block = func.block_mut(block_id);
    new_instrs.extend_from_slice(block);
    block.instrs = new_instrs;
}
