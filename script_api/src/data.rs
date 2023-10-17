use serde::{Deserialize, Serialize};

///Random data you can substitute for whatever you need
#[derive(Serialize, Deserialize)]
pub enum Data {
    DataOne(u32),
    DataTwo(i32),
}
