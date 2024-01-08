use crate::grpc::DataType;
use anyhow::Context;
use flexim_data_type::{FlData, FlDataFrame, FlImage, FlTensor2D};
use ndarray::Array2;
use polars::prelude::{IpcReader, SerReader};
use std::io::Cursor;
use std::sync::Arc;

pub(crate) fn protobuf_data_type_to_fl_data(
    data_type: DataType,
    buffer: Vec<u8>,
) -> anyhow::Result<FlData> {
    Ok(match data_type {
        DataType::Image => FlData::Image(Arc::new(FlImage::new(buffer))),
        DataType::DataFrame => FlData::DataFrame(Arc::new(dataframe_from_bytes(buffer)?)),
        DataType::Tensor2D => FlData::Tensor(Arc::new(tensor2d_from_bytes(buffer)?)),
    })
}

fn dataframe_from_bytes(buffer: Vec<u8>) -> anyhow::Result<FlDataFrame> {
    let reader = Cursor::new(buffer);
    let ipc_reader = IpcReader::new(reader);
    let df = ipc_reader.finish().context("ipc reader error")?;

    Ok(FlDataFrame::new(df))
}

fn tensor2d_from_bytes(buffer: Vec<u8>) -> anyhow::Result<FlTensor2D<f64>> {
    let reader = Cursor::new(buffer);

    let arr: Array2<f64> =
        bincode::deserialize_from(reader).context("bincode deserialize error")?;

    Ok(FlTensor2D::new(arr))
}
