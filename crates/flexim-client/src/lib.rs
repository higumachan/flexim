use anyhow::Context as _;
use flexim_connect::grpc::append_data_request::data_meta::SpecialColumn as GrpcSpecialColumn;
use flexim_connect::grpc::flexim_connect_client::FleximConnectClient;
use flexim_connect::grpc::flexim_connect_server::FleximConnectServer;
use flexim_connect::grpc::AppendDataRequest;
use flexim_connect::local_save_server::LocalSaveServerImpl;
pub use flexim_data_type::{FlDataFrameColor, FlDataFrameRectangle, FlDataFrameSegment};
use itertools::Itertools;
use polars::export::serde::Serialize;
use polars::frame::row::Row;
use polars::prelude::*;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::io::Cursor;
use std::ops::DerefMut;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use tokio::runtime::Runtime;
use tonic::codegen::tokio_stream;
use tonic::transport::{Channel, Server};

pub struct FleximClient {
    inner_client: FleximConnectClient<Channel>,
    runtime: tokio::runtime::Runtime,
    global_bag: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum SpecialColumn {
    Rectangle,
    Segment,
    Color,
}

impl From<SpecialColumn> for GrpcSpecialColumn {
    fn from(column: SpecialColumn) -> Self {
        match column {
            SpecialColumn::Rectangle => GrpcSpecialColumn::Rectangle,
            SpecialColumn::Segment => GrpcSpecialColumn::Segment,
            SpecialColumn::Color => GrpcSpecialColumn::Color,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DataValue {
    Rectangle(FlDataFrameRectangle),
    Segment(FlDataFrameSegment),
    Color(FlDataFrameColor),
    Normal(AnyValue<'static>),
}

macro_rules! impl_from_data_value {
    ($($t:ty),*) => {
        $(
            impl From<$t> for DataValue {
                fn from(value: $t) -> Self {
                    DataValue::Normal(AnyValue::from(value))
                }
            }
        )*
    };
}

macro_rules! impl_from_data_value_with_value_name {
    ($t:ty, $vn:ident) => {
        impl From<$t> for DataValue {
            fn from(value: $t) -> Self {
                DataValue::Normal(AnyValue::$vn(value.into()))
            }
        }
    };
}

macro_rules! impl_from_data_value_with_special_column {
    ($t:ty, $vn:ident) => {
        impl From<$t> for DataValue {
            fn from(value: $t) -> Self {
                DataValue::$vn(value)
            }
        }
    };
}

impl_from_data_value!(f64, f32, i64, i32);
impl_from_data_value_with_value_name!(String, Utf8Owned);
impl_from_data_value_with_value_name!(bool, Boolean);
impl_from_data_value_with_special_column!(FlDataFrameRectangle, Rectangle);
impl_from_data_value_with_special_column!(FlDataFrameSegment, Segment);
impl_from_data_value_with_special_column!(FlDataFrameColor, Color);

impl From<&DataValue> for DataType {
    fn from(value: &DataValue) -> Self {
        match value {
            DataValue::Rectangle(_) => DataType::Struct(vec![
                Field::new("x1", DataType::Float64),
                Field::new("y1", DataType::Float64),
                Field::new("x2", DataType::Float64),
                Field::new("y2", DataType::Float64),
            ]),
            DataValue::Segment(_) => DataType::Struct(vec![
                Field::new("x1", DataType::Float64),
                Field::new("y1", DataType::Float64),
                Field::new("x2", DataType::Float64),
                Field::new("y2", DataType::Float64),
            ]),
            DataValue::Color(_) => DataType::Struct(vec![
                Field::new("r", DataType::Float64),
                Field::new("g", DataType::Float64),
                Field::new("b", DataType::Float64),
            ]),
            DataValue::Normal(value) => value.into(),
        }
    }
}

impl From<DataValue> for AnyValue<'static> {
    fn from(value: DataValue) -> Self {
        match value {
            DataValue::Rectangle(value) => AnyValue::StructOwned(Box::new((
                vec![
                    AnyValue::Float64(value.x1),
                    AnyValue::Float64(value.y1),
                    AnyValue::Float64(value.x2),
                    AnyValue::Float64(value.y2),
                ],
                vec![
                    Field::new("x1", DataType::Float64),
                    Field::new("y1", DataType::Float64),
                    Field::new("x2", DataType::Float64),
                    Field::new("y2", DataType::Float64),
                ],
            ))),
            DataValue::Segment(value) => AnyValue::StructOwned(Box::new((
                vec![
                    AnyValue::Float64(value.x1),
                    AnyValue::Float64(value.y1),
                    AnyValue::Float64(value.x2),
                    AnyValue::Float64(value.y2),
                ],
                vec![
                    Field::new("x1", DataType::Float64),
                    Field::new("y1", DataType::Float64),
                    Field::new("x2", DataType::Float64),
                    Field::new("y2", DataType::Float64),
                ],
            ))),
            DataValue::Color(value) => AnyValue::StructOwned(Box::new((
                vec![
                    AnyValue::Float32(value.r),
                    AnyValue::Float32(value.g),
                    AnyValue::Float32(value.b),
                ],
                vec![
                    Field::new("r", DataType::Float64),
                    Field::new("g", DataType::Float64),
                    Field::new("b", DataType::Float64),
                ],
            ))),
            DataValue::Normal(value) => value,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RowData {
    rows: Vec<HashMap<String, DataValue>>,
}

impl RowData {
    pub fn add_row<'a, 'b>(&'a mut self) -> RowBuilder<'a>
    where
        'a: 'b,
    {
        RowBuilder::new(self)
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let schema = Schema::from_iter(
            self.rows[0]
                .iter()
                .map(|(key, value)| Field::new(key.as_str(), value.into())),
        );

        let rows = self
            .rows
            .iter()
            .map(|row| {
                let mut out_row = vec![];
                for (key, _) in schema.iter() {
                    out_row.push(row.get(&key.to_string()).unwrap().clone().into());
                }
                Row::new(out_row)
            })
            .collect_vec();

        let mut dataframe = DataFrame::from_rows_and_schema(&rows, &schema)?;

        let mut buf = Cursor::new(Vec::new());
        IpcWriter::new(&mut buf)
            .finish(&mut dataframe)
            .context("failed write to ipc")?;

        Ok(buf.into_inner())
    }
}

impl From<RowData> for Data {
    fn from(row_data: RowData) -> Self {
        Data::DataRows(row_data)
    }
}

pub struct RowBuilder<'a> {
    data: &'a mut RowData,
    columns: HashMap<String, DataValue>,
}

impl<'a> RowBuilder<'a> {
    pub fn new(data: &'a mut RowData) -> Self {
        Self {
            data,
            columns: HashMap::new(),
        }
    }

    pub fn add_column(mut self, name: &str, value: impl Into<DataValue>) -> Self {
        self.columns.insert(name.to_string(), value.into());
        self
    }
}

impl<'a> Drop for RowBuilder<'a> {
    fn drop(&mut self) {
        let Self { data, columns } = self;
        data.rows.push(columns.clone());
    }
}

pub enum Data {
    DataRows(RowData),
    DataObject(Value),
}

impl Data {
    pub fn from_serialize<T: Serialize>(value: T) -> anyhow::Result<Self> {
        Ok(Data::DataObject(serde_json::to_value(value)?))
    }
}

impl From<&Data> for flexim_connect::grpc::DataType {
    fn from(value: &Data) -> Self {
        match value {
            Data::DataRows(ref _row_data) => flexim_connect::grpc::DataType::DataFrame,
            Data::DataObject(ref _value) => flexim_connect::grpc::DataType::Object,
        }
    }
}

static CLIENT: OnceLock<Mutex<FleximClient>> = OnceLock::new();
static SERVER_RUNTIMES: Mutex<BTreeMap<u16, Runtime>> = Mutex::new(BTreeMap::new());

pub fn init_server() -> anyhow::Result<()> {
    if CLIENT.get().is_none() {
        connect_to_server(50051)
    } else {
        Ok(())
    }
}

pub fn init_localstorage(base_directory: impl AsRef<Path>) -> anyhow::Result<()> {
    let port = 50111;

    let base_directory = base_directory.as_ref();

    if !base_directory.exists() {
        std::fs::create_dir_all(base_directory)?;
    }

    SERVER_RUNTIMES
        .lock()
        .unwrap()
        .entry(port)
        .or_insert_with(move || {
            let server_impl = LocalSaveServerImpl::new(base_directory.to_path_buf());

            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap();

            runtime.spawn(async move {
                let addr = format!("[::1]:{port}").parse().unwrap();
                Server::builder()
                    .add_service(FleximConnectServer::new(server_impl))
                    .serve(addr)
                    .await
                    .unwrap();
            });

            runtime
        });
    // FIXME(higumachan): ここでスリープではなく、サーバの起動を待つべき
    std::thread::sleep(std::time::Duration::from_secs(1));

    connect_to_server(port)?;

    Ok(())
}

fn connect_to_server(port: u16) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .context("Failed to create tokio runtime")?;
    let endpoint = format!("http://[::1]:{}", port);
    let channel = runtime
        .block_on(async {
            tonic::transport::Endpoint::from_str(endpoint.as_str())?
                .connect()
                .await
        })
        .context("Failed to connect to flexim server")?;

    CLIENT
        .set(Mutex::new(FleximClient {
            inner_client: FleximConnectClient::new(channel),
            runtime,
            global_bag: None,
        }))
        .map_err(|_| anyhow::anyhow!("Failed to set client"))?;

    Ok(())
}

pub fn set_global_bag(id: u64) -> anyhow::Result<()> {
    let mut client = CLIENT
        .get()
        .context("Client not initialized")?
        .lock()
        .unwrap();

    client.global_bag = Some(id);

    Ok(())
}

pub fn create_bag(name: &str) -> anyhow::Result<u64> {
    let mut client = CLIENT
        .get()
        .context("Client not initialized")?
        .lock()
        .unwrap();
    let request = tonic::Request::new(flexim_connect::grpc::CreateBagRequest {
        name: name.to_string(),
    });

    let FleximClient {
        runtime,
        inner_client,
        ..
    } = client.deref_mut();

    let response = runtime
        .block_on(inner_client.create_bag(request))
        .context("Failed to create bag")?;

    Ok(response.into_inner().id)
}

pub fn append_data_into_global_bag(name: &str, data: Data) -> anyhow::Result<()> {
    let bag_id = CLIENT
        .get()
        .context("Client not initialized")?
        .lock()
        .unwrap()
        .global_bag
        .context("Global bag not set")?;

    append_data(bag_id, name, data)
}

pub fn append_data(bag_id: u64, name: &str, data: Data) -> anyhow::Result<()> {
    let data_type = flexim_connect::grpc::DataType::from(&data);
    let (data_bytes, special_columns) = match data {
        Data::DataRows(row_data) => (
            row_data.to_bytes()?,
            row_data.rows[0]
                .iter()
                .filter_map(|(key, value)| match value {
                    DataValue::Rectangle(_) => Some((key.to_string(), SpecialColumn::Rectangle)),
                    DataValue::Segment(_) => Some((key.to_string(), SpecialColumn::Segment)),
                    DataValue::Color(_) => Some((key.to_string(), SpecialColumn::Color)),
                    DataValue::Normal(_) => None,
                })
                .collect_vec(),
        ),
        Data::DataObject(ref value) => {
            let data_bytes = serde_json::to_vec(&value)?;
            (data_bytes, vec![])
        }
    };

    let mut client = CLIENT
        .get()
        .context("Client not initialized")?
        .lock()
        .unwrap();

    let messages = vec![
        AppendDataRequest {
            data: Some(flexim_connect::grpc::append_data_request::Data::Meta(
                flexim_connect::grpc::append_data_request::DataMeta {
                    bag_id,
                    name: name.to_string(),
                    data_type: data_type.into(),
                    special_columns: special_columns
                        .into_iter()
                        .map(|(k, s)| (k, GrpcSpecialColumn::from(s).into()))
                        .collect(),
                },
            )),
        },
        // FIXME: ここでデータを分割して送信する必要がある
        AppendDataRequest {
            data: Some(flexim_connect::grpc::append_data_request::Data::DataBytes(
                data_bytes,
            )),
        },
    ];

    let FleximClient {
        runtime,
        inner_client,
        ..
    } = client.deref_mut();

    runtime.block_on(async { inner_client.append_data(tokio_stream::iter(messages)).await })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::Data::DataObject;
    use crate::*;
    use serde_json::json;

    #[test]
    fn it_works() {
        init_localstorage("/tmp/test").unwrap();
        // init_server().unwrap();

        let bag_id = create_bag("test_from_rust").unwrap();
        let mut row_data = RowData::default();
        row_data
            .add_row()
            .add_column("Name", "nadeko".to_string())
            .add_column("Age", 14)
            .add_column(
                "Face",
                FlDataFrameRectangle {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 100.0,
                    y2: 100.0,
                },
            )
            .add_column(
                "Color",
                FlDataFrameColor {
                    r: 255.0,
                    g: 0.0,
                    b: 0.0,
                },
            );

        row_data
            .add_row()
            .add_column("Name", "koyomi".to_string())
            .add_column("Age", 17)
            .add_column(
                "Face",
                FlDataFrameRectangle {
                    x1: 200.0,
                    y1: 200.0,
                    x2: 300.0,
                    y2: 300.0,
                },
            )
            .add_column(
                "Color",
                FlDataFrameColor {
                    r: 0.0,
                    g: 255.0,
                    b: 0.0,
                },
            );
        append_data(bag_id, "test_data", Data::DataRows(row_data)).unwrap();

        append_data(
            bag_id,
            "test_object",
            DataObject(json! {
                {
                    "name": "nadeko",
                    "age": 14,
                    "face": {
                        "x1": 0.0,
                        "y1": 0.0,
                        "x2": 100.0,
                        "y2": 100.0
                    },
                    "color": {
                        "r": 255.0,
                        "g": 0.0,
                        "b": 0.0
                    }
                }
            }),
        )
        .unwrap();
    }
}
