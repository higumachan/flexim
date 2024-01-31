use flexim_connect::grpc::append_data_request::{Data, DataMeta};
use flexim_connect::grpc::flexim_connect_client::FleximConnectClient;

use flexim_connect::grpc::{
    AppendDataRequest, DataType, ListBagsRequest,
};
use std::collections::HashMap;
use tonic::codegen::tokio_stream;
use tonic::transport::{Endpoint};

const CHUNK_SIZE: usize = 128 * 1024;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Endpoint::from_static("http://[::1]:50051")
        .connect()
        .await?;
    let mut client = FleximConnectClient::new(channel);

    let request = tonic::Request::new(ListBagsRequest {});

    let response = client.list_bags(request).await?;

    println!("RESPONSE={:?}", response);

    let bag_id = response.into_inner().bag_metas.first().unwrap().id;

    let mut data_vec = vec![AppendDataRequest {
        data: Some(Data::Meta(DataMeta {
            bag_id,
            name: "test_data".to_string(),
            data_type: DataType::Image.into(),
            special_columns: HashMap::new(),
        })),
    }];
    let image_bytes = include_bytes!("../../../assets/flexim-logo-1.png");
    data_vec.extend(
        image_bytes
            .chunks(CHUNK_SIZE)
            .map(|chunk| AppendDataRequest {
                data: Some(Data::DataBytes(chunk.to_vec())),
            }),
    );

    println!("DATA_VEC_LEN={}", data_vec.len());

    let resp = client
        .append_data(tokio_stream::iter(data_vec))
        .await
        .unwrap();

    println!("RESPONSE={:?}", resp);

    Ok(())
}
