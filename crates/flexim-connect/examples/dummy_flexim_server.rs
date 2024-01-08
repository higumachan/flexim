use flexim_connect::grpc::flexim_connect_server::FleximConnectServer;
use flexim_connect::server::FleximConnectServerImpl;
use tonic::transport::Server;
use tonic::{Request, Status};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let server_impl = FleximConnectServerImpl::default();

    Server::builder()
        .add_service(FleximConnectServer::with_interceptor(
            server_impl,
            intercept,
        ))
        .serve(addr)
        .await?;

    Ok(())
}

fn intercept(mut request: Request<()>) -> Result<Request<()>, Status> {
    println!("Got a request: {:?}", request);
    Ok(request)
}
