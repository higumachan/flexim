use crate::{gen_id, Id};
use flexim_data_type::{FlDataReference};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlObjectView {
    pub id: Id,
    pub content: FlDataReference,
}

impl FlObjectView {
    pub fn new(content: FlDataReference) -> Self {
        Self {
            id: gen_id(),
            content,
        }
    }
}
