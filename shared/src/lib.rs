use serde::{Deserialize, Serialize};

pub mod models;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Env {
    #[serde(rename = "dev")]
    Dev,
    #[serde(rename = "prod")]
    Prod,
}
