use flexim_data_type::FlData;

pub trait DataVisualizable {
    fn is_visualizable(&self) -> bool;
}

impl DataVisualizable for FlData {
    fn is_visualizable(&self) -> bool {
        match self {
            FlData::Image(_) => true,
            FlData::Tensor(_) => true,
            FlData::DataFrame(_) => false,
            FlData::Object(_) => false,
        }
    }
}
