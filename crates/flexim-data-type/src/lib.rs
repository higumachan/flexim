use image::DynamicImage;
use ndarray::{Array2, Array3};
use polars::frame::DataFrame;
use std::io::Bytes;

pub struct FlImage {
    // png buffer
    pub value: Vec<u8>,
}

impl FlImage {
    pub fn new(value: Vec<u8>) -> Self {
        Self { value }
    }
}

#[derive(Debug, Clone)]
pub struct FlTensor2D<A> {
    pub value: Array2<A>,
}

impl<A> FlTensor2D<A> {
    pub fn new(value: Array2<A>) -> Self {
        Self { value }
    }
}

pub struct FlTensor3D<A> {
    pub value: Array3<A>,
}

impl<A> FlTensor3D<A> {
    pub fn new(value: Array3<A>) -> Self {
        Self { value }
    }
}

pub struct FlDataFrame {
    pub value: DataFrame,
}

impl FlDataFrame {
    pub fn new(value: DataFrame) -> Self {
        Self { value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
