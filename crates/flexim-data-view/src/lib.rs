use egui::ahash::HashMap;
use flexim_data_type::{FlData, FlDataReference};
use flexim_table_widget::{FlTable, FlTableDrawContext};
use itertools::Itertools;
use std::sync::{Arc, Mutex};

use rand::random;
use serde::{Deserialize, Serialize};

pub type Id = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlDataFrameView {
    pub id: Id,
    pub table: FlTable,
    pub view_context: Arc<Mutex<FlDataFrameViewContext>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShowColumns {
    All,
    Some(HashMap<String, usize>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlDataFrameViewContext {
    pub show_columns: ShowColumns,
}

impl From<FlDataFrameViewContext> for FlTableDrawContext {
    fn from(view: FlDataFrameViewContext) -> Self {
        let draw_columns = match view.show_columns {
            ShowColumns::All => flexim_table_widget::ShowColumns::All,
            ShowColumns::Some(columns) => {
                let mut columns = columns.iter().map(|(k, v)| (k.clone(), *v)).collect_vec();
                columns.sort_by_key(|(_, v)| *v);
                let columns = columns.into_iter().map(|(k, _)| k).collect();
                flexim_table_widget::ShowColumns::Some(columns)
            }
        };

        Self {
            show_columns: draw_columns,
        }
    }
}

impl FlDataFrameView {
    pub fn new(data_reference: FlDataReference) -> Self {
        Self {
            id: gen_id(),
            table: FlTable::new(data_reference),
            view_context: Arc::new(Mutex::new(FlDataFrameViewContext {
                show_columns: ShowColumns::All,
            })),
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
