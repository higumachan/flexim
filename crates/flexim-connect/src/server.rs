use crate::grpc::flexim_connect_server::FleximConnect;
use crate::grpc::*;
use crate::utility::protobuf_data_type_to_fl_data;
use flexim_storage::Storage;
use polars::export::num;
use std::sync::Arc;
use tonic::codegen::Body;
use tonic::{Request, Response, Status, Streaming};

#[derive(Default)]
pub struct FleximConnectServerImpl {
    storage: Arc<Storage>,
}

#[tonic::async_trait]
impl FleximConnect for FleximConnectServerImpl {
    async fn create_bag(
        &self,
        request: Request<CreateBagRequest>,
    ) -> Result<Response<CreateBagResponse>, Status> {
        let name = request.into_inner().name;
        let bag_id = self.storage.create_bag(name.clone());
        Ok(Response::new(CreateBagResponse {
            id: bag_id.into_inner(),
            name,
        }))
    }

    async fn append_data(
        &self,
        request: Request<Streaming<AppendDataRequest>>,
    ) -> Result<Response<AppendDataResponse>, Status> {
        let mut streaming = request.into_inner();
        let mut meta = None;
        let mut buffer = vec![];

        while let Some(req) = streaming.message().await? {
            println!("Got message = {:?}", req);
            match req.data {
                Some(append_data_request::Data::Meta(mes_meta)) => {
                    println!("Got meta = {:?}", meta);
                    meta = Some(mes_meta);
                }
                Some(append_data_request::Data::DataBytes(data)) => {
                    println!("Got data = {:?}", data);
                    buffer.extend(data);
                }
                _ => {
                    unreachable!()
                }
            }
        }

        if let Some(meta) = meta {
            let data_size = buffer.len() as u64;
            let data = protobuf_data_type_to_fl_data(
                DataType::try_from(meta.data_type)
                    .map_err(|e| Status::invalid_argument(e.to_string()))?,
                buffer,
            )
            .map_err(|e| Status::internal(e.to_string()))?;
            let data_id = data.id();

            self.storage
                .insert_data(flexim_storage::BagId::new(meta.bag_id), data)
                .map_err(|e| Status::internal(e.to_string()))?;

            Ok(Response::new(AppendDataResponse {
                bag_id: meta.bag_id,
                data_id: data_id as u64,
                data_size,
            }))
        } else {
            Err(Status::invalid_argument("meta not found"))
        }
    }
}
