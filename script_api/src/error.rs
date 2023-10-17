use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScriptError {
    #[error("Device cannot be switched On/Off")]
    InvalidDeviceActionOnOff,

    #[error("Device cannot have items")]
    InvalidDeviceActionItems,

    #[error("Device does not have an On/Off state to get")]
    InvalidDeviceGetState,
}
