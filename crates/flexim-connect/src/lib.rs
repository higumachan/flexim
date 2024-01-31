mod client;
pub mod server;
mod utility;

pub mod grpc {
    tonic::include_proto!("flexim_connect");
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn it_works() {}
}
