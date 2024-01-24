use egui::ahash::{HashMap, HashSet, HashSetExt};

use egui::{Align, Sense, Slider, Ui};
use egui_extras::{Column, TableBuilder};
use flexim_data_type::FlDataFrame;
use polars::prelude::*;
use std::ops::{BitAnd, DerefMut};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct FlTable {
    pub dataframe: Arc<FlDataFrame>,
    state: Arc<FlTableState>,
}

impl FlTable {
    pub fn new(dataframe: Arc<FlDataFrame>) -> Self {
        Self {
            dataframe: dataframe.clone(),
            state: Arc::new(FlTableState::new(&dataframe.value)),
        }
    }

    pub fn draw(&self, ui: &mut Ui) {
        let dataframe = &self.dataframe.value;
        let columns = dataframe.get_column_names();
        let dataframe = self.computed_dataframe();

        let mut builder = TableBuilder::new(ui).vscroll(true).striped(true);

        builder = builder.column(Column::auto().clip(true).resizable(true));
        for _col in &columns {
            builder = builder.column(Column::auto().clip(true).resizable(true));
        }
        let state = self.state();
        let mut selected = *state.selected.lock().unwrap();
        let builder = if let Some(selected) = selected {
            log::info!("selected: {}", selected);
            builder.scroll_to_row(selected as usize, Some(Align::Center))
        } else {
            builder
        };
        let builder = builder.sense(Sense::click());
        builder
            .header(80.0, |mut header| {
                for col in &columns {
                    header.col(|ui| {
                        ui.heading(col.to_string());
                        state.filters.get(&col.to_string()).unwrap().draw(ui);
                    });
                }
            })
            .body(|mut body| {
                for row_idx in 0..dataframe.height() {
                    body.row(32.0, |mut row| {
                        // クリックしたらハイライトに追加する
                        let d = dataframe
                            .column("__FleximRowId")
                            .unwrap()
                            .get(row_idx)
                            .unwrap()
                            .extract::<u32>()
                            .unwrap() as u64;
                        let mut highlight = state.highlight.lock().unwrap();
                        let selected = state.selected.lock().unwrap();

                        if highlight.contains(&d) {
                            row.set_selected(true);
                        }
                        if let Some(selected) = *selected {
                            if selected == d {
                                row.set_selected(true);
                            }
                        }
                        for c in &columns {
                            let (_, r) = row.col(|ui| {
                                let c = dataframe
                                    .column(&c)
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
                }
            });
    }

    pub fn state(&self) -> Arc<FlTableState> {
        self.state.clone()
    }

    pub fn computed_dataframe(&self) -> DataFrame {
        let state = self.state();
        let dataframe = &self.dataframe.value;
        let columns = dataframe.get_column_names();
        let dataframe = dataframe.with_row_count("__FleximRowId", None).unwrap();
        let mut col_filter_mask = std::iter::repeat(true)
            .take(dataframe.height() as usize)
            .collect::<BooleanChunked>();

        for col in &columns {
            let filter = state.filters.get(*col).unwrap().filter.lock().unwrap();
            let series = dataframe.column(col).unwrap();
            if let Some(filter) = filter.as_ref() {
                if let Some(m) = filter.apply(series) {
                    col_filter_mask = col_filter_mask.bitand(m);
                }
            }
        }
        dataframe.filter(&col_filter_mask).unwrap()
    }
}

type ColumnName = String;

#[derive(Debug, Clone)]
pub struct FlTableState {
    pub filters: HashMap<ColumnName, ColumnFilter>,
    pub highlight: Arc<Mutex<HashSet<u64>>>,
    pub selected: Arc<Mutex<Option<u64>>>,
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
            highlight: Arc::new(Mutex::new(HashSet::new())),
            selected: Arc::new(Mutex::new(None)),
        }
    }
}

#[derive(Debug, Clone)]
struct Aggregated {
    min_max: Option<(f64, f64)>,
    unique: Option<Vec<String>>,
    dtype: DataType,
}

#[derive(Debug, Clone)]
pub struct ColumnFilter {
    filter: Arc<Mutex<Option<Filter>>>,
    aggregated: Arc<Aggregated>,
}

impl ColumnFilter {
    pub fn draw(&self, ui: &mut Ui) {
        match self.filter.lock().unwrap().deref_mut() {
            Some(Filter::RangeFilter { min, max }) => {
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
            Some(Filter::SearchFilter(search)) => {
                ui.text_edit_singleline(search);
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
                filter: Arc::new(Mutex::new(Some(Filter::RangeFilter { min, max }))),
            }
        } else if dtype == &DataType::Utf8 {
            Self {
                aggregated,
                filter: Arc::new(Mutex::new(Some(Filter::SearchFilter(String::new())))),
            }
        } else if let DataType::Categorical(_d) = dtype {
            Self {
                aggregated,
                filter: Arc::new(Mutex::new(Some(Filter::CategoricalFilter(HashSet::new())))),
            }
        } else {
            Self {
                aggregated,
                filter: Arc::new(Mutex::new(None)),
            }
        }
    }
}

#[derive(Debug, Clone)]
enum Filter {
    SearchFilter(String),
    RangeFilter { min: f64, max: f64 },
    CategoricalFilter(HashSet<String>),
}

impl Filter {
    fn apply(&self, series: &Series) -> Option<BooleanChunked> {
        match self {
            Filter::SearchFilter(search) => Some(series.utf8().ok()?.contains(search, true).ok()?),
            Filter::RangeFilter { min, max } => {
                let series = series.cast(&DataType::Float64).ok()?;
                let series = series.f64().ok()?;
                Some(series.gt_eq(*min).bitand(series.lt_eq(*max)))
            }
            Filter::CategoricalFilter(categories) => {
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
            .collect(),
    )
}
