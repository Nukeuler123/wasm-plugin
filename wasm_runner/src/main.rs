use std::{env, fs, time::Instant};

use wasm_runner::WasmVM;
fn main() {
    let args: Vec<String> = env::args().collect();
    let code = fs::read_to_string(args.get(1).unwrap()).unwrap();

    let mut vm = WasmVM::new(code).unwrap();


    for _ in 0..10 {
        let time = Instant::now();
        let outputs = vm.run_tick(Vec::default()).unwrap();
        println!("Debug text: \n{}", vm.read_debug_string().unwrap());
        println!(
            "Took {} μs with a instruction cost of: {}",
            time.elapsed().as_micros(),
            vm.get_instructions_used().unwrap()
        );
        println!("outputs: {:#?}", outputs);
    }
}
