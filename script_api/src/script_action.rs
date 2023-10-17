use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ScriptAction {
    ActionOne,
    ActionTwo,
    ActionThree,
}
