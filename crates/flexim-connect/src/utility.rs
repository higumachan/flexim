use crate::grpc::append_data_request::data_meta::SpecialColumn;
use crate::grpc::append_data_request::DataMeta;
use crate::grpc::DataType;
use anyhow::Context;
use flexim_data_type::{
    FlData, FlDataFrame, FlDataFrameSpecialColumn, FlImage, FlObject, FlTensor2D,
};
use polars::prelude::{IpcReader, SerReader};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

pub(crate) fn protobuf_data_type_to_fl_data(
    meta: DataMeta,
    buffer: Vec<u8>,
) -> anyhow::Result<FlData> {
    Ok(match meta.data_type.try_into()? {
        DataType::Image => FlData::Image(Arc::new(FlImage::try_from_bytes(buffer)?)),
        DataType::DataFrame => FlData::DataFrame(Arc::new(dataframe_from_bytes(
            meta.special_columns,
            buffer,
        )?)),
        DataType::Tensor2D => FlData::Tensor(Arc::new(tensor2d_from_bytes(buffer)?)),
        DataType::Object => FlData::Object(Arc::new(object_from_bytes(buffer)?)),
    })
}

fn dataframe_from_bytes(
    special_columns: HashMap<String, i32>,
    buffer: Vec<u8>,
) -> anyhow::Result<FlDataFrame> {
    let reader = Cursor::new(buffer);
    let ipc_reader = IpcReader::new(reader);
    let df = ipc_reader.finish().context("ipc reader error")?;

    let special_columns = special_columns
        .into_iter()
        .map(|(k, v)| Ok((k, special_column_convert(SpecialColumn::try_from(v)?)?)))
        .collect::<anyhow::Result<HashMap<_, _>>>()?;

    Ok(FlDataFrame::new(df, special_columns))
}

fn special_column_convert(
    special_column: SpecialColumn,
) -> anyhow::Result<FlDataFrameSpecialColumn> {
    match special_column {
        SpecialColumn::Rectangle => Ok(FlDataFrameSpecialColumn::Rectangle),
        SpecialColumn::Segment => Ok(FlDataFrameSpecialColumn::Segment),
        SpecialColumn::Color => Ok(FlDataFrameSpecialColumn::Color),
        _ => anyhow::bail!("unknown special column"),
    }
}

fn tensor2d_from_bytes(buffer: Vec<u8>) -> anyhow::Result<FlTensor2D<f64>> {
    let reader = Cursor::new(buffer);

    let arr: FlTensor2D<f64> =
        bincode::deserialize_from(reader).context("bincode deserialize error")?;

    Ok(arr)
}

fn object_from_bytes(buffer: Vec<u8>) -> anyhow::Result<FlObject> {
    let reader = Cursor::new(buffer);

    let obj: Value = serde_json::from_reader(reader).context("serde_json deserialize error")?;

    Ok(FlObject::new(obj))
}
