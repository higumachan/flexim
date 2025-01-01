use crate::visualize::{DataRender, FlDataFrameViewRender};
use anyhow::Context;
use egui::ahash::HashMap;
use egui::{CollapsingHeader, ScrollArea, Style, Ui};
use flexim_data_type::{FlDataFrame, FlDataReference};
use flexim_data_view::object::FlObjectView;
use flexim_data_view::{FlDataFrameView, Id, ShowColumns};
use flexim_storage::Bag;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
pub enum DataView {
    FlDataFrameView(FlDataFrameView),
    FlObjectView(FlObjectView),
}

impl DataView {
    pub fn id(&self) -> Id {
        match self {
            Self::FlDataFrameView(v) => v.id(),
            Self::FlObjectView(v) => v.id(),
        }
    }

    pub fn draw(&self, ui: &mut Ui, bag: &Bag) {
        match self {
            Self::FlDataFrameView(v) => v.draw(ui, bag),
            Self::FlObjectView(v) => v.draw(ui, bag),
        }
    }

    pub fn visualizeable_attributes(&self, bag: &Bag) -> Vec<String> {
        match self {
            Self::FlDataFrameView(v) => v.visualizeable_attributes(bag),
            Self::FlObjectView(v) => v.visualizeable_attributes(bag),
        }
    }

    pub fn create_visualize(&self, attribute: String) -> Arc<DataRender> {
        match self {
            Self::FlDataFrameView(v) => v.create_visualize(attribute),
            Self::FlObjectView(v) => v.create_visualize(attribute),
        }
    }

    pub fn reference(&self) -> FlDataReference {
        match self {
            Self::FlDataFrameView(v) => v.table.data_reference.clone(),
            Self::FlObjectView(v) => v.content.clone(),
        }
    }

    pub fn config_panel(&self, ui: &mut Ui, bag: &Bag) {
        match self {
            Self::FlDataFrameView(v) => v.config_panel(ui, bag),
            Self::FlObjectView(v) => v.config_panel(ui, bag),
        }
    }
}

pub trait DataViewable {
    fn id(&self) -> Id;
    fn draw(&self, ui: &mut Ui, bag: &Bag);
    fn visualizeable_attributes(&self, bag: &Bag) -> Vec<String>;
    fn create_visualize(&self, attribute: String) -> Arc<DataRender>;
    fn config_panel(&self, ui: &mut Ui, bag: &Bag);
}

impl DataViewable for FlDataFrameView {
    fn id(&self) -> Id {
        self.id
    }

    fn draw(&self, ui: &mut Ui, bag: &Bag) {
        puffin::profile_function!();
        ScrollArea::horizontal()
            .enable_scrolling(true)
            .max_width(ui.available_width())
            .min_scrolled_width(ui.available_width())
            .drag_to_scroll(true)
            .show(ui, |ui| {
                let view_context = self.view_context.lock().unwrap();

                self.table.draw(ui, bag, &view_context.clone().into());
            });
    }

    fn visualizeable_attributes(&self, bag: &Bag) -> Vec<String> {
        let dataframe = self.table.dataframe(bag);
        let FlDataFrame {
            value: dataframe,
            special_columns,
            ..
        } = dataframe.as_ref().unwrap().as_ref();
        dataframe
            .fields()
            .iter()
            .filter(|field| {
                special_columns
                    .get(&field.name().to_string())
                    .is_some_and(|v| v.visualizable_attribute())
            })
            .map(|field| field.name().to_string())
            .collect()
    }

    fn create_visualize(&self, attribute: String) -> Arc<DataRender> {
        Arc::new(FlDataFrameViewRender::new(self.clone(), attribute).into())
    }

    fn config_panel(&self, ui: &mut Ui, bag: &Bag) {
        ui.label("DataFrame");
        CollapsingHeader::new("Config")
            .default_open(true)
            .show(ui, |ui| {
                let dataframe = self.table.dataframe(bag).expect("DataFrame not found");
                let column_names = dataframe.value.get_column_names();

                let mut view_context = self.view_context.lock().unwrap();
                for (i, column_name) in column_names.iter().enumerate() {
                    let column_name = column_name.to_string();
                    let mut checked = match &view_context.show_columns {
                        ShowColumns::All => true,
                        ShowColumns::Some(columns) => {
                            columns.contains_key(&column_name.to_string())
                        }
                    };
                    if ui.checkbox(&mut checked, column_name.to_string()).changed() {
                        if checked {
                            match &mut view_context.show_columns {
                                ShowColumns::All => {}
                                ShowColumns::Some(columns) => {
                                    columns.insert(column_name.to_string(), i);
                                }
                            }
                        } else {
                            match &mut view_context.show_columns {
                                ShowColumns::All => {
                                    let mut all_columns: HashMap<_, _> = column_names
                                        .iter()
                                        .enumerate()
                                        .map(|(v, k)| (k.to_string(), v))
                                        .collect();
                                    all_columns.remove(&column_name);
                                    view_context.show_columns = ShowColumns::Some(all_columns);
                                }
                                ShowColumns::Some(columns) => {
                                    columns.remove(&column_name.to_string());
                                }
                            }
                        }
                    }
                }
            });
    }
}

impl DataViewable for FlObjectView {
    fn id(&self) -> Id {
        self.id
    }

    fn draw(&self, ui: &mut Ui, bag: &Bag) {
        let object = bag
            .data_by_reference(&self.content)
            .expect("Object not found")
            .as_object()
            .expect("Mismatched data type");

        let code = serde_json::to_string_pretty(&object.value)
            .context("Failed to serialize object")
            .expect("Failed to serialize object");

        let theme =
            egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), &Style::default());
        ScrollArea::both()
            .auto_shrink(false)
            .max_width(f32::INFINITY)
            .show(ui, |ui| {
                egui_extras::syntax_highlighting::code_view_ui(ui, &theme, code.as_str(), "json");
            });
    }

    fn visualizeable_attributes(&self, _bag: &Bag) -> Vec<String> {
        vec![]
    }

    fn create_visualize(&self, _attribute: String) -> Arc<DataRender> {
        unreachable!()
    }

    fn config_panel(&self, _ui: &mut Ui, _bag: &Bag) {
        // empty implementation
    }
}
