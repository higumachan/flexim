use flexim_connect::grpc::flexim_connect_server::FleximConnectServer;
use flexim_connect::local_save_server::LocalSaveServerImpl;
use flexim_data_type::FlTensor2D;
use ndarray::Array2;
use numpy::PyReadonlyArrayDyn;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::collections::BTreeMap;
use std::sync::Mutex;
use tokio::runtime::Runtime;
use tonic::transport::Server;

static SERVER_RUNTIMES: Mutex<BTreeMap<u16, Runtime>> = Mutex::new(BTreeMap::new());

/// A Python module implemented in Rust.
#[pymodule]
fn _flexim_py_lib(_py: Python, m: &PyModule) -> PyResult<()> {
    /// Pythonのndarrayを受け取りその内容をndarrayとして解釈できるバイト列に変換する
    #[pyfn(m)]
    fn tensor2d_to_bytes<'py>(
        _py: Python<'py>,
        tensor2d: PyReadonlyArrayDyn<'py, f32>,
        offset: (u64, u64),
    ) -> PyResult<&'py PyBytes> {
        let array: Array2<f64> = tensor2d
            .as_array()
            .mapv(f64::from)
            .into_dimensionality()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let array = FlTensor2D::new(array, offset);

        bincode::serialize(&array)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
            .map(|v| PyBytes::new(_py, &v))
    }

    #[pyfn(m)]
    fn start_localstorage_server(_py: Python<'_>, base_directory: &str, port: u16) -> PyResult<()> {
        let base_directory = std::path::Path::new(base_directory);

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

        Ok(())
    }
    Ok(())
}
