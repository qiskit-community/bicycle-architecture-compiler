use ftir;

use pyo3::prelude::*;

#[pymodule]
fn pyftir<'py>(_py: Python<'py>, m: &Bound<'py, PyModule>) -> PyResult<()> {

    #[pyfn(m)]
    fn do_something(_py: Python, a: f32, b: f32) -> PyResult<f32> {
        let out = ftir::do_something(a, b);
        Ok(out)
    }

    Ok(())
}

