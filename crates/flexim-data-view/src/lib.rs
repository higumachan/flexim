pub mod object;

use egui::ahash::{HashMap, HashSet};
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
    #[serde(default)]
    pub view_context: Arc<Mutex<FlDataFrameViewContext>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ShowColumns {
    #[default]
    All,
    Some(HashSet<String>, Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlDataFrameViewContext {
    pub show_columns: ShowColumns,
}

impl From<FlDataFrameViewContext> for FlTableDrawContext {
    fn from(view: FlDataFrameViewContext) -> Self {
        let draw_columns = match view.show_columns {
            ShowColumns::All => flexim_table_widget::ShowColumns::All,
            ShowColumns::Some(has_column, columns) => flexim_table_widget::ShowColumns::Some(
                columns
                    .iter()
                    .filter(|c| has_column.contains(*c))
                    .map(|c| c.clone())
                    .collect(),
            ),
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
        matches!(self, FlData::DataFrame(_) | FlData::Object(_))
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
