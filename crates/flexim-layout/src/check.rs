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
            .or_insert_with(|| {
                let mut t = HashSet::new();
                t.insert(data.generation);
                t
            });
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
    use crate::pane::{Pane, PaneContent};
    use egui_tiles::Tree;
    use flexim_data_type::FlImage;
    use flexim_data_visualize::visualize::{DataRender, FlImageRender};
    use flexim_storage::{BagId, ManagedData};
    use rstest::{fixture, rstest};
    use std::sync::Arc;

    #[fixture]
    fn simple_bag() -> Bag {
        Bag {
            id: BagId::new(0),
            name: "bag".to_string(),
            created_at: chrono::Utc::now(),
            data_list: vec![ManagedData {
                name: "data".to_string(),
                generation: 0,
                data: flexim_data_type::FlData::Image(Arc::new(
                    FlImage::try_from_bytes(include_bytes!("../../../assets/logo.png").to_vec())
                        .unwrap(),
                )),
            }],
            generation_counter: std::collections::HashMap::new(),
        }
    }

    #[rstest]
    #[case("data", GenerationSelector::Latest, true)]
    #[case("data", GenerationSelector::Generation(0), true)]
    #[case("data", GenerationSelector::Generation(2), false)]
    #[case("data_t", GenerationSelector::Latest, false)]
    fn simple_case(
        simple_bag: Bag,
        #[case] data_name: &str,
        #[case] generation: GenerationSelector,
        #[case] expected: bool,
    ) {
        assert_eq!(
            check_applicable(
                &simple_bag,
                &FlLayout::new(
                    "layout".to_string(),
                    Tree::new_horizontal(
                        "test_tree",
                        vec![Pane::new(
                            "pane1".to_string(),
                            PaneContent::Visualize(Arc::new(DataRender::Image(
                                FlImageRender::new(FlDataReference {
                                    name: data_name.to_string(),
                                    generation,
                                    data_type: flexim_data_type::FlDataType::Image,
                                })
                            ))),
                        )],
                    ),
                ),
            ),
            expected
        );
    }
}
