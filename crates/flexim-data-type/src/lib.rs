use anyhow::{bail, Context};

use ndarray::{Array2, Array3};

use image::{load_from_memory_with_format, EncodableLayout, ImageDecoder};
use polars::frame::DataFrame;
use polars::prelude::*;
use rand::random;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

type Id = u64;

pub trait FlDataTrait {
    fn id(&self) -> Id;
}

#[derive(Debug, Clone)]
pub enum FlData {
    Image(Arc<FlImage>),
    Tensor(Arc<FlTensor2D<f64>>),
    DataFrame(Arc<FlDataFrame>),
}

impl FlData {
    pub fn id(&self) -> Id {
        match self {
            Self::Image(v) => v.id(),
            Self::Tensor(v) => v.id(),
            Self::DataFrame(v) => v.id(),
        }
    }
}

impl From<FlImage> for FlData {
    fn from(value: FlImage) -> Self {
        Self::Image(Arc::new(value))
    }
}

impl From<FlTensor2D<f64>> for FlData {
    fn from(value: FlTensor2D<f64>) -> Self {
        Self::Tensor(Arc::new(value))
    }
}

impl From<FlDataFrame> for FlData {
    fn from(value: FlDataFrame) -> Self {
        Self::DataFrame(Arc::new(value))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlImage {
    pub id: Id,
    pub value: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

impl FlImage {
    pub fn new(value: Vec<u8>, width: usize, height: usize) -> Self {
        Self {
            id: gen_id(),
            value,
            width,
            height,
        }
    }

    pub fn try_from_bytes(value: Vec<u8>) -> anyhow::Result<Self> {
        let (width, height) = {
            let v = value.as_bytes();
            let decoder = image::codecs::png::PngDecoder::new(v).context("png decoder error")?;
            decoder.dimensions()
        };
        Ok(Self {
            id: gen_id(),
            value,
            width: width as usize,
            height: height as usize,
        })
    }
}

impl FlDataTrait for FlImage {
    fn id(&self) -> Id {
        self.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlTensor2D<A> {
    pub id: Id,
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

impl FlDataTrait for FlTensor2D<f64> {
    fn id(&self) -> Id {
        self.id
    }
}

#[derive(Debug, Clone)]
pub struct FlTensor3D<A> {
    pub id: Id,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlDataFrame {
    pub id: Id,
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

impl FlDataTrait for FlDataFrame {
    fn id(&self) -> Id {
        self.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlDataFrameRectangle {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl<'a> TryFrom<AnyValue<'a>> for FlDataFrameRectangle {
    type Error = anyhow::Error;

    fn try_from(value: AnyValue<'a>) -> Result<Self, Self::Error> {
        let mut x1 = None;
        let mut y1 = None;
        let mut x2 = None;
        let mut y2 = None;
        let mut update_func = |field: &Field, value: AnyValue| {
            if !field.dtype.is_float() {
                bail!("Expected float field, found {:?}", field.dtype);
            }
            match field.name().as_str() {
                "x1" => x1 = Some(value.try_extract().context("Expected float")?),
                "y1" => y1 = Some(value.try_extract().context("Expected float")?),
                "x2" => x2 = Some(value.try_extract().context("Expected float")?),
                "y2" => y2 = Some(value.try_extract().context("Expected float")?),
                _ => bail!("Unknown field {:?}", field.name()),
            }
            Ok(())
        };

        let value = value.into_static()?;
        match value {
            AnyValue::StructOwned(s) => {
                for (field, value) in s.1.iter().zip(s.0) {
                    update_func(field, value)?;
                }
            }
            _ => bail!("Expected struct, found {:?}", value),
        }
        Ok(Self {
            x1: x1.context("Missing field x1")?,
            y1: y1.context("Missing field y1")?,
            x2: x2.context("Missing field x2")?,
            y2: y2.context("Missing field y2")?,
        })
    }
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

    pub fn validate_fields(fields: &[Field]) -> bool {
        let field_map: HashMap<_, _> = fields.iter().map(|f| (f.name.as_str(), &f.dtype)).collect();
        ["x1", "y1", "x2", "y2"].into_iter().all(|key| {
            if let Some(dt) = field_map.get(key) {
                dt.is_float()
            } else {
                false
            }
        })
    }
}

fn gen_id() -> Id {
    random()
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
