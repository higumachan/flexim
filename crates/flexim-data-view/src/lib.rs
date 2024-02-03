use egui::Vec2;
use flexim_data_type::{FlData, FlDataFrame, FlDataReference};
use flexim_table_widget::FlTable;
use std::collections::HashMap;

use rand::random;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type Id = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlDataFrameView {
    pub id: Id,
    pub table: FlTable,
}

impl FlDataFrameView {
    pub fn new(data_reference: FlDataReference) -> Self {
        Self {
            id: gen_id(),
            table: FlTable::new(data_reference),
        }
    }
}

pub trait DataViewCreatable {
    fn data_view_creatable(&self) -> bool;
}

impl DataViewCreatable for FlData {
    fn data_view_creatable(&self) -> bool {
        matches!(self, FlData::DataFrame(_))
    }
}

fn gen_id() -> Id {
    random()
}
#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
