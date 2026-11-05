// Copyright contributors to the Bicycle Architecture Compiler project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bicycle_compiler::{PathArchitecture, language::AnglePrecision};
use pyo3::{
    exceptions::{PyTypeError, PyValueError},
    intern,
    prelude::*,
    sync::PyOnceLock,
    types::{PyDict, PyString, PyType},
};

use crate::bicycle::iter_pbc_circuit;
use crate::table::{CodeType, get_measurement_table};
use qiskit_pyo3_ffi::{self as ffi, qk_circuit_num_qubits};

static QUANTUM_CIRCUIT: PyOnceLock<Py<PyType>> = PyOnceLock::new();

fn qc_type(py: Python<'_>) -> &Py<PyType> {
    QUANTUM_CIRCUIT.get_or_init(py, || {
        py.import("qiskit")
            .expect("qiskit module import failed")
            .getattr("QuantumCircuit")
            .expect("qiskit.QuantumCircuit import failed")
            .cast::<PyType>()
            .expect("failed casting to QuantumCircuit type")
            .clone()
            .unbind()
    })
}

/// Take a Qiskit ``QuantumCircuit`` instance to compile and run numerics on each instruction.
///
/// Args:
///     circuit: The ``QuantumCircuit`` instance.
///     code: The code, either ``"gross"`` or ``"two_gross"``.
///     physical_error: The physical accuracy, either ``"1e-3"`` or ``"1e-4"``.
///     epsilon: The target accuracy for gridsynth. Pass this as string!
///     measurement_table: The path to the measurement table. Generates it at that path
///         (which defaults to the local directory) if it does not exist.
///
/// Returns:
///     A dictionary containing the accumulated error rates and instruction counts.
///
/// Raises:
///     TypeError: If the input is not a ``QuantumCircuit``.
///     TypeError: If any instruction in the circuit is not a ``PauliProductRotationGate`` or a
///         ``PauliProductMeasurement``.
///     ValueError: If the ``code`` is unsupported.
///     ValueError: If the ``physical_error`` is unsupported.
#[pyfunction]
#[pyo3(signature = (circuit, code, physical_error, epsilon, measurement_table = None))]
fn estimate<'py>(
    circuit: Bound<'py, PyAny>,
    code: Bound<'py, PyString>,
    physical_error: Bound<'py, PyString>,
    epsilon: Bound<'py, PyString>,
    measurement_table: Option<Bound<'py, PyString>>,
) -> PyResult<Bound<'py, PyDict>> {
    let py = circuit.py();

    if !circuit.is_instance(qc_type(py).bind(py))? {
        return Err(PyTypeError::new_err("Input is not of type QuantumCircuit"));
    }

    let code = match code.to_str()? {
        "gross" => CodeType::Gross,
        "two_gross" => CodeType::TwoGross,
        _ => return Err(PyValueError::new_err("Invalid code")),
    };

    let model = match (&code, physical_error.to_str()?) {
        (CodeType::Gross, "1e-3") => bicycle_numerics::model::GROSS_1E3,
        (CodeType::Gross, "1e-4") => bicycle_numerics::model::GROSS_1E4,
        (CodeType::TwoGross, "1e-3") => bicycle_numerics::model::TWO_GROSS_1E3,
        (CodeType::TwoGross, "1e-4") => bicycle_numerics::model::TWO_GROSS_1E4,
        _ => return Err(PyValueError::new_err("Invalid physical error")),
    };

    let circuit_data = circuit.getattr(intern!(py, "_data"))?;
    let circuit_ptr = unsafe { ffi::qk_circuit_borrow_from_python(circuit_data.as_ptr()) };

    let num_qubits = unsafe { qk_circuit_num_qubits(circuit_ptr) };
    let arch = PathArchitecture::for_qubits(num_qubits as usize);
    let accuracy = AnglePrecision::lit(epsilon.to_str()?);

    let measurement_table = measurement_table.as_ref().map(|s| s.to_str()).transpose()?;
    let table = get_measurement_table(code, measurement_table);

    let bicycle = iter_pbc_circuit(circuit_ptr)
        .map_err(|e| PyTypeError::new_err(e.to_owned()))?
        .map(|op| op.compile(&arch, &table, accuracy));

    // Accumulate all operation counts
    let mut idles = 0;
    let mut t_injs = 0;
    let mut automorphisms = 0;
    let mut measurements = 0;
    let mut joint_measurements = 0;
    let mut total_error = 0.0;
    let mut end_time = 0;

    for data in bicycle_numerics::run_numerics(bicycle, arch, model) {
        idles += data.idles;
        t_injs += data.t_injs;
        automorphisms += data.automorphisms;
        measurements += data.measurements;
        joint_measurements += data.joint_measurements;

        // the iterator does not know it's length so we just overwrite the total error
        // and end time with the most up to data value
        total_error = data.total_error;
        end_time = data.end_time;
    }

    let results = PyDict::new(py);
    results.set_item("total_error", total_error)?;
    results.set_item("end_time", end_time)?;

    let counts = PyDict::new(py);
    counts.set_item("t_injs", t_injs)?;
    counts.set_item("automorphisms", automorphisms)?;
    counts.set_item("measurements", measurements)?;
    counts.set_item("joint_measurements", joint_measurements)?;
    counts.set_item("idles", idles)?;

    results.set_item("counts", counts)?;
    Ok(results)
}

#[pymodule]
mod numerics {
    use pyo3::prelude::*;
    use qiskit_pyo3_ffi as ffi;

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        ffi::qk_import(m.py())?;
        Ok(())
    }

    #[pymodule_export]
    use super::estimate;
}
