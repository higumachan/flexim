pub mod check;
pub mod pane;

use crate::pane::Pane;
use egui::Id;
use egui_tiles::Tree;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct FlLayout {
    pub id: Id,
    pub name: String,
    pub tree: Tree<Pane>,
}

impl FlLayout {
    pub fn new(name: String, tree: Tree<Pane>) -> Self {
        let id = Id::new(name.clone()).with(tree.id());
        Self { id, name, tree }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_deserializing_previously_layout_file() {
        // これが落ちる場合には、assets/test_data/layout/test-layout.json の内容を確認してください。
        // FIXME: 現時点では後方互換性は強く意識していないのでこのテストが落ちた場合は状況を確認した上でファイルを修正するのでも良い場合があります。
        let layout_file = include_bytes!("../../../assets/test_data/layout/test-layout.json");

        let _layout: Vec<FlLayout> = serde_json::from_slice(layout_file).unwrap();
    }
}
