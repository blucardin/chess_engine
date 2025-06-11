use bincode::{config, Decode, Encode};
use bincode::config::Configuration;
use serde::{Deserialize, Serialize};

pub const CONFIG: Configuration = config::standard();

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone)]
pub struct MovesAndLabelRaw {
    pub id: i64,
    pub white_winner: bool,
    #[serde(with = "serde_bytes")]
    pub move_list: Vec<u8>,
}