use egui::ahash::{HashMap, HashSet, HashSetExt};
use egui::Key::{N, T};
use egui::WidgetType::TextEdit;
use egui::{Slider, Ui};
use egui_extras::{Column, TableBuilder};
use polars::prelude::*;
use std::ops::{BitAnd, DerefMut};
use std::sync::Mutex;

pub struct FlTable {
    dataframe: Arc<DataFrame>,
}

impl FlTable {
    pub fn new(dataframe: Arc<DataFrame>) -> Self {
        Self { dataframe }
    }

    pub fn draw(&self, ui: &mut Ui) {
        let id = ui.make_persistent_id("fl_table");
        let mut state = ui.memory_mut(|mem| {
            mem.data
                .get_persisted::<FlTableState>(id)
                .unwrap_or_else(|| {
                    let t = FlTableState::new(&self.dataframe);
                    mem.data.insert_persisted(id, t.clone());
                    t
                })
        });

        let columns = self.dataframe.get_column_names();
        let mut builder = TableBuilder::new(ui).vscroll(true).striped(true);
        let mut col_filter_mask = std::iter::repeat(true)
            .take(self.dataframe.height())
            .collect::<BooleanChunked>();

        for col in &columns {
            builder = builder.column(Column::auto().resizable(true));
            let filter = state.filters.get(*col).unwrap().filter.lock().unwrap();
            let series = self.dataframe.column(col).unwrap();
            if let Some(filter) = filter.as_ref() {
                if let Some(m) = filter.apply(series) {
                    col_filter_mask = col_filter_mask.bitand(m);
                }
            }
        }
        let dataframe = self.dataframe.filter(&col_filter_mask).unwrap();
        builder
            .header(32.0, |mut header| {
                for col in columns {
                    header.col(|ui| {
                        ui.heading(col.to_string());
                        state.filters.get_mut(&col.to_string()).unwrap().draw(ui);
                    });
                }
            })
            .body(|mut body| {
                for row_idx in 0..dataframe.height() {
                    body.row(32.0, |mut row| {
                        let draw = dataframe.get_row(row_idx).unwrap();

                        for c in draw.0 {
                            row.col(|ui| {
                                ui.label(c.to_string());
                            });
                        }
                    });
                }
            });
    }
}

type ColumnName = String;

#[derive(Debug, Clone)]
pub struct FlTableState {
    pub filters: HashMap<ColumnName, ColumnFilter>,
}

impl FlTableState {
    fn new(data_frame: &DataFrame) -> Self {
        println!("new");
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
    pub fn draw(&mut self, ui: &mut Ui) {
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
        } else if let DataType::Categorical(d) = dtype {
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
