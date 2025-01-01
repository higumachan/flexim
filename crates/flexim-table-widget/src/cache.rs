use egui::ahash::HashMap;
use egui::util::cache::CacheTrait;
use egui::Id;
use polars::frame::DataFrame;

#[derive(Debug, Clone)]
pub struct Calculating {
    generation: u64,
    previous: Option<DataFrame>,
    previous_generation: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum CacheState {
    Ready {
        generation: u64,
        dataframe: DataFrame,
    },
    Calculating(Calculating),
}

pub enum DataFramePoll<T> {
    Ready(T),
    Pending,
}

#[derive(Default)]
pub struct FilteredDataFrameCache {
    cached_dataframes: HashMap<Id, CacheState>,
}

impl FilteredDataFrameCache {
    pub fn insert_computed(&mut self, id: Id, generation: u64, dataframe: DataFrame) {
        self.cached_dataframes
            .entry(id)
            .and_modify({
                let dataframe = dataframe.clone();
                |t| match t {
                    CacheState::Ready {
                        generation: gen, ..
                    } => {
                        if *gen == generation {
                            *t = CacheState::Ready {
                                generation,
                                dataframe,
                            };
                        }
                    }
                    CacheState::Calculating(calc) => {
                        if calc.generation == generation {
                            *t = CacheState::Ready {
                                generation,
                                dataframe,
                            };
                        } else if calc.previous_generation.unwrap_or(0) < generation {
                            *t = CacheState::Calculating(Calculating {
                                generation: calc.generation,
                                previous: Some(dataframe),
                                previous_generation: Some(generation),
                            });
                        }
                    }
                }
            })
            .or_insert_with({
                let dataframe = dataframe.clone();
                move || CacheState::Ready {
                    generation,
                    dataframe,
                }
            });
    }

    pub fn insert_calculating(&mut self, id: Id) -> u64 {
        let previous = self.cached_dataframes.remove(&id);
        let calc = if let Some(prev) = previous {
            match prev {
                CacheState::Ready {
                    generation,
                    dataframe,
                } => Calculating {
                    generation: generation.wrapping_add(1),
                    previous: Some(dataframe),
                    previous_generation: Some(generation),
                },
                CacheState::Calculating(mut calc) => Calculating {
                    generation: calc.generation.wrapping_add(1),
                    previous: calc.previous.take(),
                    previous_generation: Some(calc.generation),
                },
            }
        } else {
            Calculating {
                generation: 0,
                previous: None,
                previous_generation: None,
            }
        };
        let gen = calc.generation;
        self.cached_dataframes
            .insert(id, CacheState::Calculating(calc));
        gen
    }

    pub fn get(&self, id: Id) -> Option<DataFramePoll<DataFrame>> {
        self.cached_dataframes.get(&id).map(|t| match t {
            CacheState::Ready { dataframe, .. } => DataFramePoll::Ready(dataframe.clone()),
            CacheState::Calculating(calc) => {
                if let Some(prev) = &calc.previous {
                    DataFramePoll::Ready(prev.clone())
                } else {
                    DataFramePoll::Pending
                }
            }
        })
    }
}

impl CacheTrait for FilteredDataFrameCache {
    fn update(&mut self) {}

    fn len(&self) -> usize {
        self.cached_dataframes.len()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
