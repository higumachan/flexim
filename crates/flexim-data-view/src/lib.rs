use flexim_data_type::{FlData, FlDataReference};
use flexim_table_widget::FlTable;

use rand::random;
use serde::{Deserialize, Serialize};

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
