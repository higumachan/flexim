use crate::grpc::flexim_connect_server::FleximConnect;
use crate::grpc::list_bags_response::BagMeta;
use crate::grpc::*;
use crate::utility::protobuf_data_type_to_fl_data;
use flexim_storage::{BagId, Storage, StorageQuery};
use std::fs::File;
use std::io::BufWriter;
use std::ops::Deref;
use std::path::PathBuf;

use std::sync::Arc;
use tonic::{Request, Response, Status, Streaming};

pub struct LocalSaveServerImpl {
    base_directory: PathBuf,
    storage: Arc<Storage>,
}

impl LocalSaveServerImpl {
    pub fn new(base_directory: PathBuf) -> Self {
        Self {
            base_directory,
            storage: Arc::new(Storage::default()),
        }
    }
}

#[tonic::async_trait]
impl FleximConnect for LocalSaveServerImpl {
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
            match req.data {
                Some(append_data_request::Data::Meta(mes_meta)) => {
                    meta = Some(mes_meta);
                }
                Some(append_data_request::Data::DataBytes(data)) => {
                    buffer.extend(data);
                }
                _ => {
                    unreachable!()
                }
            }
        }

        // TODO(higumachan): エラーハンドリングをちゃんとやっていく
        if let Some(meta) = meta {
            let data_size = buffer.len() as u64;
            let data = protobuf_data_type_to_fl_data(meta.clone(), buffer)
                .map_err(|e| Status::internal(e.to_string()))?;
            let data_id = data.id();

            let bag_id = BagId::new(meta.bag_id);
            self.storage
                .insert_data(bag_id, meta.name, data)
                .map_err(|e| Status::internal(e.to_string()))?;

            let bag = self.storage.get_bag(bag_id).unwrap();

            let bag = bag.read().unwrap();
            let mut writer = BufWriter::new(
                File::create(
                    self.base_directory
                        .join(format!("{}_{}.bag", bag.name, bag.created_at)),
                )
                .unwrap(),
            );
            bincode::serialize_into(&mut writer, bag.deref()).unwrap();

            Ok(Response::new(AppendDataResponse {
                bag_id: meta.bag_id,
                data_id: data_id as u64,
                data_size,
            }))
        } else {
            Err(Status::invalid_argument("meta not found"))
        }
    }

    async fn list_bags(
        &self,
        _request: Request<ListBagsRequest>,
    ) -> Result<Response<ListBagsResponse>, Status> {
        let bags = self
            .storage
            .list_bags()
            .map_err(|e| Status::internal(e.to_string()))?;
        let bag_metas = bags
            .into_iter()
            .map(|bag| {
                let bag = bag.read().unwrap();
                BagMeta {
                    id: bag.id.into_inner(),
                    name: bag.name.clone(),
                    number_of_data: bag.data_list.len() as u64,
                    created_at: bag.created_at.to_string(),
                }
            })
            .collect();
        Ok(Response::new(ListBagsResponse { bag_metas }))
    }
}
