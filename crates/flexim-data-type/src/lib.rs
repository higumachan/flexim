use anyhow::{anyhow, Context};

use ndarray::{Array2, Array3};

use image::{EncodableLayout, ImageDecoder};
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

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum FlDataType {
    Image,
    Tensor,
    DataFrame,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn data_type(&self) -> FlDataType {
        match self {
            Self::Image(_) => FlDataType::Image,
            Self::Tensor(_) => FlDataType::Tensor,
            Self::DataFrame(_) => FlDataType::DataFrame,
        }
    }

    pub fn as_image(&self) -> Option<Arc<FlImage>> {
        match self {
            Self::Image(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn as_tensor(&self) -> Option<Arc<FlTensor2D<f64>>> {
        match self {
            Self::Tensor(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn as_data_frame(&self) -> Option<Arc<FlDataFrame>> {
        match self {
            Self::DataFrame(v) => Some(v.clone()),
            _ => None,
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
    pub special_columns: HashMap<String, FlDataFrameSpecialColumn>,
}

impl FlDataFrame {
    pub fn new(
        value: DataFrame,
        special_columns: HashMap<String, FlDataFrameSpecialColumn>,
    ) -> Self {
        Self {
            id: gen_id(),
            value,
            special_columns,
        }
    }
}

impl FlDataTrait for FlDataFrame {
    fn id(&self) -> Id {
        self.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlDataFrameSpecialColumn {
    Rectangle,
    Segment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlDataFrameRectangle {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum FlShapeConvertError {
    #[error("Null value")]
    NullValue,
    #[error("Unhandled error {0}")]
    UnhandledError(#[from] anyhow::Error),
}

impl<'a> TryFrom<AnyValue<'a>> for FlDataFrameRectangle {
    type Error = FlShapeConvertError;

    fn try_from(value: AnyValue<'a>) -> Result<Self, Self::Error> {
        let mut x1 = None;
        let mut y1 = None;
        let mut x2 = None;
        let mut y2 = None;
        let mut update_func = |field: &Field, value: AnyValue| {
            if !field.dtype.is_float() {
                return Err(Self::Error::UnhandledError(anyhow!(
                    "Expected float field, found {:?}",
                    field.dtype
                )));
            }
            match field.name().as_str() {
                "x1" => {
                    x1 = if !value.is_nested_null() {
                        Some(Some(value.try_extract().context("Expected float")?))
                    } else {
                        Some(None)
                    }
                }
                "y1" => {
                    y1 = if !value.is_nested_null() {
                        Some(Some(value.try_extract().context("Expected float")?))
                    } else {
                        Some(None)
                    }
                }
                "x2" => {
                    x2 = if !value.is_nested_null() {
                        Some(Some(value.try_extract().context("Expected float")?))
                    } else {
                        Some(None)
                    }
                }
                "y2" => {
                    y2 = if !value.is_nested_null() {
                        Some(Some(value.try_extract().context("Expected float")?))
                    } else {
                        Some(None)
                    }
                }
                _ => {
                    return Err(Self::Error::UnhandledError(anyhow!(
                        "Unknown field {:?}",
                        field.name()
                    )));
                }
            }
            Ok(())
        };

        let value = value.into_static().context("Failed to convert to static")?;

        match value {
            AnyValue::StructOwned(s) => {
                for (field, value) in s.1.iter().zip(s.0) {
                    update_func(field, value)?;
                }
            }
            _ => {
                return Err(Self::Error::UnhandledError(anyhow!(
                    "Expected struct, found {:?}",
                    value
                )));
            }
        }

        Ok(Self {
            x1: x1
                .context("Missing field x1")?
                .ok_or(Self::Error::NullValue)?,
            y1: y1
                .context("Missing field y1")?
                .ok_or(Self::Error::NullValue)?,
            x2: x2
                .context("Missing field x2")?
                .ok_or(Self::Error::NullValue)?,
            y2: y2
                .context("Missing field y2")?
                .ok_or(Self::Error::NullValue)?,
        })
    }
}

impl FlDataFrameRectangle {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlDataFrameSegment {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl<'a> TryFrom<AnyValue<'a>> for FlDataFrameSegment {
    type Error = FlShapeConvertError;

    fn try_from(value: AnyValue<'a>) -> Result<Self, Self::Error> {
        let mut x1 = None;
        let mut y1 = None;
        let mut x2 = None;
        let mut y2 = None;
        let mut update_func = |field: &Field, value: AnyValue| {
            if !field.dtype.is_float() {
                return Err(Self::Error::UnhandledError(anyhow!(
                    "Expected float field, found {:?}",
                    field.dtype
                )));
            }
            let value = if !value.is_nested_null() {
                Some(Some(value.try_extract().context("Expected float")?))
            } else {
                Some(None)
            };
            match field.name().as_str() {
                "x1" => {
                    x1 = value;
                }
                "y1" => {
                    y1 = value;
                }
                "x2" => {
                    x2 = value;
                }
                "y2" => {
                    y2 = value;
                }
                _ => {
                    return Err(Self::Error::UnhandledError(anyhow!(
                        "Unknown field {:?}",
                        field.name()
                    )));
                }
            }
            Ok(())
        };

        let value = value.into_static().context("Failed to convert to static")?;
        match value {
            AnyValue::StructOwned(s) => {
                for (field, value) in s.1.iter().zip(s.0) {
                    update_func(field, value)?;
                }
            }
            _ => {
                return Err(Self::Error::UnhandledError(anyhow!(
                    "Expected struct, found {:?}",
                    value
                )));
            }
        }
        Ok(Self {
            x1: x1
                .context("Missing field x1")?
                .ok_or(Self::Error::NullValue)?,
            y1: y1
                .context("Missing field y1")?
                .ok_or(Self::Error::NullValue)?,
            x2: x2
                .context("Missing field x2")?
                .ok_or(Self::Error::NullValue)?,
            y2: y2
                .context("Missing field y2")?
                .ok_or(Self::Error::NullValue)?,
        })
    }
}

impl FlDataFrameSegment {
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

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum GenerationSelector {
    Latest,
    Generation(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct FlDataReference {
    pub name: String,
    pub generation: GenerationSelector,
    pub data_type: FlDataType,
}

impl FlDataReference {
    pub fn new(name: String, generation: GenerationSelector, data_type: FlDataType) -> Self {
        Self {
            name,
            generation,
            data_type,
        }
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
