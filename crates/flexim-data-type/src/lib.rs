use image::DynamicImage;
use ndarray::{Array2, Array3};
use polars::frame::DataFrame;
use polars::prelude::{
    AnyValue, ChunkedSet, DataType, Field, NamedFrom, PolarsResult, StructChunked,
};
use rand::{random, thread_rng, Rng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum FlData {
    Image(FlImage),
    Tensor(FlTensor2D<f64>),
    DataFrame(FlDataFrame),
}

impl From<FlImage> for FlData {
    fn from(value: FlImage) -> Self {
        Self::Image(value)
    }
}

impl From<FlTensor2D<f64>> for FlData {
    fn from(value: FlTensor2D<f64>) -> Self {
        Self::Tensor(value)
    }
}

impl From<FlDataFrame> for FlData {
    fn from(value: FlDataFrame) -> Self {
        Self::DataFrame(value)
    }
}

#[derive(Debug, Clone)]
pub struct FlImage {
    pub id: usize,
    // png buffer
    pub value: Vec<u8>,
}

impl FlImage {
    pub fn new(value: Vec<u8>) -> Self {
        Self {
            id: gen_id(),
            value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlTensor2D<A> {
    pub id: usize,
    pub value: Array2<A>,
}

impl<A> FlTensor2D<A> {
    pub fn new(value: Array2<A>) -> Self {
        Self {
            id: gen_id(),
            value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlTensor3D<A> {
    pub id: usize,
    pub value: Array3<A>,
}

impl<A> FlTensor3D<A> {
    pub fn new(value: Array3<A>) -> Self {
        Self {
            id: gen_id(),
            value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlDataFrame {
    pub id: usize,
    pub value: DataFrame,
}

impl FlDataFrame {
    pub fn new(value: DataFrame) -> Self {
        Self {
            id: gen_id(),
            value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlDataFrameRectangle {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl FlDataFrameRectangle {
    pub fn fields() -> Vec<Field> {
        vec![
            Field::new("x1", DataType::Float64),
            Field::new("y1", DataType::Float64),
            Field::new("x2", DataType::Float64),
            Field::new("y2", DataType::Float64),
        ]
    }
}

fn gen_id() -> usize {
    random()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
