use script_api::*;

pub struct Script{

}

impl Script {
    pub fn new() -> Self {
        Self {  }
    }

    pub fn run(&mut self) {
        debug!("Hello, world!");
        action_one();
    }
}