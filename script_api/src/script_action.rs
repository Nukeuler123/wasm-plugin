use serde::{Deserialize, Serialize};

//Script actions, when your script makes a decision it will add one of thes
//To the output buffer so that your main program can actually perform them
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ScriptAction {
    ActionOne,
    ActionTwo,
    ActionThree,
}
