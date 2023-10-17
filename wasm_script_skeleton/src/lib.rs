mod script;

pub use script_api::*;

static mut SCRIPT: Option<script::Script> = None;

///The actual function that runs the script
#[no_mangle]
pub unsafe fn export_run() {
    script_api::panic::reset();
    unsafe {
        //If the script has already been created just run
        let script_opt = &mut SCRIPT;
        if let Some(script) = script_opt {
            script.run();
        } 
        else {
            //First run, install panic hook and initalizes the script before running
            script_api::panic::install();
            SCRIPT = Some(script::Script::new());
            if let Some(script) = script_opt {
                script.run();
            } else {
                panic!("Script failed to run when initalized");
            }
        }
    }
}
