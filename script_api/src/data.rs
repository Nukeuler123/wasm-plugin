use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Data {
    DataOne(u32),
    DataTwo(i32),
}
