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
mod tests {}
