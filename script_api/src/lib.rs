use data::Data;
pub use debug::*;
pub use script_action::ScriptAction;

mod data;
mod debug;
pub mod panic;
mod script_action;

pub const MAX_INPUT_SIZE: usize = 2048;
pub const MAX_OUTPUT_SIZE: usize = 2048;

///External VM runner cann pull data out of this buffer to figure out what the script wants to do
#[no_mangle]
static mut SCRIPT_OUTPUT_BUFFER: [u8; MAX_OUTPUT_SIZE] = [0; MAX_OUTPUT_SIZE];

///External VM runner can put data into this buffer for the script to act on
#[no_mangle]
static mut DATA_INPUT_BUFFER: [u8; MAX_INPUT_SIZE] = [0; MAX_INPUT_SIZE];

pub type Inputs = Vec<Data>;
pub type Outputs = Vec<ScriptAction>;

///Writes data to the output buffer, should NOT be used by the script directly, though that depends on your usecase
pub(crate) fn write_action_to_buffer(action: ScriptAction) {
    //Parse the actions already in the buffer and append the new one onto it
    let mut actions = read_output_buffer();
    actions.push(action);

    let mut encoded_actions = bincode::serialize::<Outputs>(&actions).unwrap();

    //Creates a new body with the size at the start and the actual content appended at the end
    let mut body = (encoded_actions.len() as u32).to_le_bytes().to_vec();
    body.append(&mut encoded_actions);

    //Takes the body created and loads it into the fixed buffer
    let system_state = unsafe { &mut SCRIPT_OUTPUT_BUFFER };
    system_state[..body.len()].copy_from_slice(&body);
}
pub fn action_one() {
    write_action_to_buffer(ScriptAction::ActionOne);
}

pub fn action_two() {
    write_action_to_buffer(ScriptAction::ActionTwo);
}

pub fn action_three() {
    write_action_to_buffer(ScriptAction::ActionThree);
}

fn read_output_buffer() -> Outputs {
    let script_output = unsafe { &mut SCRIPT_OUTPUT_BUFFER };
    read_buffer(script_output.as_mut())
}

///Gets all data that has been put in the script's buffer
pub fn read_input_buffer() -> Vec<Inputs> {
    let script_input = unsafe { &mut DATA_INPUT_BUFFER };
    read_buffer(script_input.as_mut())
}

fn read_buffer<'a, T>(buffer: &'a mut [u8]) -> Vec<T>
where
    T: serde::de::Deserialize<'a>,
{
    //Split array between the size u32 and the rest of the body
    let (size_bytes, body) = buffer.split_at(4);

    //Get size
    let mut u32_buffer: [u8; 4] = [0; 4];
    u32_buffer.copy_from_slice(size_bytes);
    let size = u32::from_le_bytes(u32_buffer);

    if size == 0 {
        return Vec::new();
    }

    //Find the actual content of buffer and parse
    let content: &[u8] = &body[0..size as usize];

    bincode::deserialize(content).unwrap()
}
