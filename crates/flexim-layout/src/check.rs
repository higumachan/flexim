use crate::pane::PaneContent;
use crate::FlLayout;
use egui::ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use egui_tiles::Tile;
use flexim_data_type::{FlDataReference, GenerationSelector};
use flexim_storage::Bag;

pub fn check_applicable(bag: &Bag, layout: &FlLayout) -> bool {
    let references = collect_references(layout);

    let mut index_name: HashMap<String, HashSet<u64>> = HashMap::new();
    for data in &bag.data_list {
        index_name
            .entry(data.name.clone())
            .and_modify(|s| {
                s.insert(data.generation);
            })
            .or_insert_with(HashSet::new);
    }

    for reference in references {
        match reference.generation {
            GenerationSelector::Generation(generation) => {
                if !index_name
                    .get(&reference.name)
                    .map(|s| s.contains(&generation))
                    .unwrap_or(false)
                {
                    return false;
                }
            }
            GenerationSelector::Latest => {
                if !index_name.contains_key(&reference.name) {
                    return false;
                }
            }
        }
    }

    true
}

fn collect_references(layout: &FlLayout) -> HashSet<FlDataReference> {
    HashSet::from_iter(
        layout
            .tree
            .tiles
            .iter()
            .filter_map(|(_, tile)| match tile {
                Tile::Pane(pane) => Some(pane),
                _ => None,
            })
            .map(|pane| pane.content.reference()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_case() {}
}
