pub mod cache;

use egui::ahash::{HashMap, HashSet, HashSetExt};

use crate::cache::{DataFramePoll, FilteredDataFrameCache};

use egui::{Align, ComboBox, Id, Sense, Slider, Ui};
use egui_extras::{Column, TableBuilder};
use flexim_data_type::{FlDataFrame, FlDataTrait};
use itertools::Itertools;
use polars::prelude::*;
use rand::random;
use serde::{Deserialize, Serialize};
use std::ops::{BitAnd, Deref, DerefMut};

use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlTable {
    id: Id,
    pub dataframe: Arc<FlDataFrame>,
    pub state: Arc<Mutex<FlTableState>>,
    previous_state: Option<FlTableState>,
}

impl FlTable {
    pub fn new(dataframe: Arc<FlDataFrame>) -> Self {
        Self {
            id: Id::new("FlTable")
                .with(dataframe.id())
                .with(random::<u64>()),
            dataframe: dataframe.clone(),
            state: Arc::new(Mutex::new(FlTableState::new(&dataframe.value))),
            previous_state: None,
        }
    }

    pub fn draw(&self, ui: &mut Ui) {
        puffin::profile_function!();
        if self.previous_state.is_none()
            || self.previous_state.as_ref().unwrap() != self.state.lock().unwrap().deref()
        {
            let generation = ui.ctx().memory_mut(move |mem| {
                let cache = mem.caches.cache::<FilteredDataFrameCache>();
                cache.insert_calculating(self.id)
            });
            let dataframe = self.dataframe.clone();
            let state = self.state.clone().lock().unwrap().clone();
            let ctx = ui.ctx().clone();
            let id = self.id;
            std::thread::spawn(move || {
                let dataframe = compute_dataframe(&dataframe.value, &state);
                ctx.memory_mut(move |mem| {
                    let cache = mem.caches.cache::<FilteredDataFrameCache>();
                    cache.insert_computed(id, generation, dataframe);
                });
            });
        }

        let dataframe = &self.dataframe.value;
        let columns = dataframe.get_column_names();
        let dataframe = ui.ctx().memory_mut(|mem| {
            let cache = mem.caches.cache::<FilteredDataFrameCache>();
            cache.get(self.id).unwrap()
        });

        if let DataFramePoll::Ready(dataframe) = dataframe {
            let mut builder = TableBuilder::new(ui).vscroll(true).striped(true);

            builder = builder.column(Column::auto().clip(true).resizable(true));
            for _col in &columns {
                builder = builder.column(Column::auto().clip(true).resizable(true));
            }
            let mut state = self.state.lock().unwrap();
            let selected = &mut state.selected;
            let builder = if let Some(selected) = selected {
                log::info!("selected: {}", *selected);
                builder.scroll_to_row(*selected as usize, Some(Align::Center))
            } else {
                builder
            };
            let builder = builder.sense(Sense::click());
            builder
                .header(80.0, |mut header| {
                    for col in &columns {
                        header.col(|ui| {
                            ui.heading(col.to_string());
                            let filter = state.filters.get_mut(&col.to_string()).unwrap();
                            filter.draw(Id::new(self.id).with(col), ui);
                        });
                    }
                })
                .body(|body| {
                    body.rows(32.0, dataframe.height(), |mut row| {
                        let row_idx = row.index();

                        // クリックしたらハイライトに追加する
                        puffin::profile_scope!("row");
                        let d = dataframe
                            .column("__FleximRowId")
                            .unwrap()
                            .get(row_idx)
                            .unwrap()
                            .extract::<u32>()
                            .unwrap() as u64;
                        let selected = state.selected;
                        let highlight = &mut state.deref_mut().highlight;

                        if highlight.contains(&d) {
                            row.set_selected(true);
                        }
                        if let Some(selected) = selected {
                            if selected == d {
                                row.set_selected(true);
                            }
                        }
                        for c in &columns {
                            let (_, _r) = row.col(|ui| {
                                let c = dataframe
                                    .column(c)
                                    .unwrap()
                                    .get(row_idx)
                                    .unwrap()
                                    .to_string();
                                ui.label(c);
                            });
                        }
                        if row.response().clicked() {
                            if highlight.contains(&d) {
                                highlight.remove(&d);
                            } else {
                                highlight.insert(d);
                            }
                        }
                    });
                });
        } else {
            ui.label("Loading...");
        }
    }

    pub fn computed_dataframe(&self, ui: &mut Ui) -> DataFramePoll<DataFrame> {
        let dataframe = ui.ctx().memory_mut(|mem| {
            let cache = mem.caches.cache::<FilteredDataFrameCache>();
            cache.get(self.id).unwrap()
        });

        dataframe
    }
}

fn compute_dataframe(dataframe: &DataFrame, state: &FlTableState) -> DataFrame {
    let columns = dataframe.get_column_names();
    let dataframe = dataframe.with_row_count("__FleximRowId", None).unwrap();
    let mut col_filter_mask = std::iter::repeat(true)
        .take(dataframe.height())
        .collect::<BooleanChunked>();

    for col in &columns {
        let filter = state.filters.get(*col).unwrap().filter.as_ref();
        let series = dataframe.column(col).unwrap();
        if let Some(filter) = filter.as_ref() {
            if let Some(m) = filter.apply(series) {
                col_filter_mask = col_filter_mask.bitand(m);
            }
        }
    }
    dataframe.filter(&col_filter_mask).unwrap()
}

type ColumnName = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlTableState {
    pub filters: HashMap<ColumnName, ColumnFilter>,
    pub highlight: HashSet<u64>,
    pub selected: Option<u64>,
}

impl FlTableState {
    fn new(data_frame: &DataFrame) -> Self {
        FlTableState {
            filters: data_frame
                .iter()
                .map(|series| {
                    (
                        series.0.name().to_string(),
                        ColumnFilter::from_series(series),
                    )
                })
                .collect(),
            highlight: HashSet::new(),
            selected: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Aggregated {
    min_max: Option<(f64, f64)>,
    unique: Option<Vec<String>>,
    dtype: DataType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnFilter {
    filter: Option<Filter>,
    aggregated: Arc<Aggregated>,
}

impl ColumnFilter {
    pub fn draw(&mut self, id: Id, ui: &mut Ui) {
        match &mut self.filter {
            Some(Filter::Range { min, max }) => {
                let range = self.aggregated.min_max.map(|(min, max)| min..=max).unwrap();

                let slider = Slider::new(min, range.clone()).text("min");
                let slider = if self.aggregated.dtype.is_integer() {
                    slider.integer()
                } else {
                    slider
                };
                ui.add(slider);
                let slider = Slider::new(max, range.clone()).text("max");
                let slider = if self.aggregated.dtype.is_integer() {
                    slider.integer()
                } else {
                    slider
                };
                ui.add(slider);
            }
            Some(Filter::Search(search)) => {
                ui.text_edit_singleline(search);
            }
            Some(Filter::Categorical(categories)) => {
                let cat = categories
                    .as_ref()
                    .and_then(|t| t.iter().next().cloned())
                    .unwrap_or("".to_string());
                ComboBox::from_id_source(id)
                    .selected_text(cat)
                    .show_ui(ui, |ui| {
                        let mut selected = None;
                        for cat in self.aggregated.unique.as_ref().unwrap() {
                            ui.selectable_value(&mut selected, Some(cat.clone()), cat);
                        }
                        if let Some(selected) = selected {
                            let mut current_categories = HashSet::new();
                            current_categories.insert(selected);
                            *categories = Some(current_categories);
                        } else {
                            *categories = None;
                        }
                    });
            }
            _ => {}
        }
    }

    fn from_series(series: &Series) -> Self {
        let dtype = series.dtype();
        let aggregated = Aggregated {
            min_max: series.cast(&DataType::Float64).ok().and_then(|t| {
                let series = t.f64().ok()?;
                let min = series.min()?;
                let max = series.max()?;

                Some((min, max))
            }),
            unique: unique_series(series),
            dtype: series.dtype().clone(),
        };

        let aggregated = Arc::new(aggregated);
        if dtype.is_numeric() {
            let (min, max) = aggregated.min_max.unwrap();
            Self {
                aggregated,
                filter: Some(Filter::Range { min, max }),
            }
        } else if dtype == &DataType::Boolean {
            Self {
                aggregated,
                filter: Some(Filter::Categorical(None)),
            }
        } else if dtype == &DataType::Utf8 {
            Self {
                aggregated,
                filter: Some(Filter::Search(String::new())),
            }
        } else if let DataType::Categorical(_d) = dtype {
            Self {
                aggregated,
                filter: Some(Filter::Categorical(None)),
            }
        } else {
            Self {
                aggregated,
                filter: None,
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum Filter {
    Search(String),
    Range { min: f64, max: f64 },
    Categorical(Option<HashSet<String>>),
}

impl Filter {
    fn apply(&self, series: &Series) -> Option<BooleanChunked> {
        match self {
            Filter::Search(search) => Some(series.utf8().ok()?.contains(search, true).ok()?),
            Filter::Range { min, max } => {
                let series = series.cast(&DataType::Float64).ok()?;
                let series = series.f64().ok()?;
                Some(series.gt_eq(*min).bitand(series.lt_eq(*max)))
            }
            Filter::Categorical(Some(categories)) => {
                let series = series.cast(&DataType::Utf8).unwrap();
                let series = series.utf8().unwrap();
                Some(
                    series
                        .into_iter()
                        .map(|t: Option<&str>| {
                            if let Some(t) = t {
                                categories.contains(t)
                            } else {
                                false
                            }
                        })
                        .collect(),
                )
            }
            Filter::Categorical(None) => None,
        }
    }
}

fn unique_series(series: &Series) -> Option<Vec<String>> {
    let series = series.cast(&DataType::Utf8).ok()?;

    Some(
        series
            .utf8()
            .ok()?
            .into_iter()
            .filter_map(|t| t.map(|s| s.to_string()))
            .unique()
            .collect(),
    )
}
