use crate::{gen_id, FlDataFrameViewContext, Id};
use flexim_data_type::{FlDataReference, FlObject};
use flexim_table_widget::FlTable;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

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
