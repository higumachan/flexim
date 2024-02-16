use ndarray::Array2;
use numpy::PyReadonlyArrayDyn;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// A Python module implemented in Rust.
#[pymodule]
fn flexim_py(_py: Python, m: &PyModule) -> PyResult<()> {
    /// Pythonのndarrayを受け取りその内容をndarrayとして解釈できるバイト列に変換する
    #[pyfn(m)]
    fn tensor2d_to_bytes<'py>(
        _py: Python<'py>,
        tensor2d: PyReadonlyArrayDyn<'py, f32>,
    ) -> PyResult<&'py PyBytes> {
        let array: Array2<f64> = tensor2d
            .as_array()
            .mapv(f64::from)
            .into_dimensionality()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        bincode::serialize(&array)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
            .map(|v| PyBytes::new(_py, &v))
    }
    Ok(())
}
