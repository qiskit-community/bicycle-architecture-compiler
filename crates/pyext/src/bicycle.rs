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

use bicycle_common::Pauli;
use bicycle_compiler::language::PbcOperation;
use hashbrown::HashSet;

use qiskit_pyo3_ffi::{self as ffi, QkOperationKind};

fn local_to_global_pauli(z: &[bool], x: &[bool], qubits: &[u32], num_qubits: usize) -> Vec<Pauli> {
    let mut basis = vec![Pauli::I; num_qubits];

    for ((zi, xi), qi) in z.iter().zip(x.iter()).zip(qubits.iter()) {
        if *qi as usize >= num_qubits {
            panic!(
                "Qubit index ({}) larger than num_qubits ({})!",
                qi, num_qubits
            );
        }

        let pauli = match (*zi, *xi) {
            (true, true) => Pauli::Y,
            (false, true) => Pauli::X,
            (true, false) => Pauli::Z,
            (false, false) => Pauli::I,
        };
        basis[*qi as usize] = pauli;
    }

    basis
}

pub(crate) fn iter_pbc_circuit(
    circuit: *const ffi::QkCircuit,
) -> Result<Box<dyn Iterator<Item = PbcOperation>>, &'static str> {
    if circuit.is_null() {
        return Ok(Box::new(::std::iter::empty()));
    }

    // SAFETY: Circuit is a valid pointer and not null.
    let num_symbols = unsafe { ffi::qk_circuit_num_param_symbols(circuit) };
    if num_symbols != 0 {
        return Err("Circuit has unbound symbols.");
    }

    // SAFETY: Circuit is a valid pointer and not null.
    let counts = unsafe { ffi::qk_circuit_count_ops(circuit) };
    let valid_names: HashSet<&str> = ["pauli_product_rotation", "pauli_product_measurement"].into();
    for i in 0..counts.len {
        let count =
            unsafe { counts.data.wrapping_add(i).as_ref() }.expect("Opcount index is in range");
        // SAFETY: Qiskit guarantees the name str is valid and nul terminated.
        let name = unsafe { ::std::ffi::CStr::from_ptr(count.name) };
        if !valid_names.contains(&name.to_str().map_err(|_e| "Failed gate name conversion")?) {
            return Err("Circuit has invalid instructions.");
        }
    }

    // SAFETY: Circuit is a valid pointer and not null.
    let num_qubits = unsafe { ffi::qk_circuit_num_qubits(circuit) } as usize;
    // SAFETY: Circuit is a valid pointer and not null.
    let num_inst = unsafe { ffi::qk_circuit_num_instructions(circuit) };

    let iter = (0..num_inst).map(move |i| {
        let mut inst = ::std::mem::MaybeUninit::<ffi::QkCircuitInstruction>::uninit();
        // SAFETY: Circuit is a valid pointer and not null, as is the instruction. The index
        // is in scope.
        unsafe {
            ffi::qk_circuit_get_instruction(circuit, i, inst.as_mut_ptr());
        }
        // SAFETY: The instruction was just initialized.
        let inst = unsafe { inst.assume_init() };
        // SAFETY: The instruction promises the data is readable for sufficient elements.
        let qubits = unsafe { ::std::slice::from_raw_parts(inst.qubits, inst.num_qubits as usize) };

        // SAFETY: Circuit is a valid pointer and not null and the index is in scope.
        let kind = unsafe { ffi::qk_circuit_instruction_kind(circuit, i) };
        if kind == QkOperationKind::PauliProductRotation {
            let mut op = ::std::mem::MaybeUninit::<ffi::QkPauliProductRotation>::uninit();
            // SAFETY: Circuit is a valid pointer and not null, as is the instruction. The index
            // is in scope.
            unsafe { ffi::qk_circuit_inst_pauli_product_rotation(circuit, i, op.as_mut_ptr()) };
            // SAFETY: The instruction was just initialized.
            let op = unsafe { op.assume_init() };
            // SAFETY: The instruction promises the data is readable for sufficient elements.
            let z = unsafe { ::std::slice::from_raw_parts(op.z, op.len) };
            // SAFETY: The instruction promises the data is readable for sufficient elements.
            let x = unsafe { ::std::slice::from_raw_parts(op.x, op.len) };

            let basis = local_to_global_pauli(z, x, qubits, num_qubits);

            // SAFETY: We checked the circuit has no parameters.
            let angle = unsafe { ffi::qk_param_as_real(op.angle) };

            PbcOperation::rotation(basis, angle)
        } else if kind == QkOperationKind::PauliProductMeasurement {
            let mut op = ::std::mem::MaybeUninit::<ffi::QkPauliProductMeasurement>::uninit();
            // SAFETY: Circuit is a valid pointer and not null, as is the instruction. The index
            // is in scope.
            unsafe { ffi::qk_circuit_inst_pauli_product_measurement(circuit, i, op.as_mut_ptr()) };
            // SAFETY: The instruction was just initialized.
            let op = unsafe { op.assume_init() };
            // SAFETY: The instruction promises the data is readable for sufficient elements.
            let z = unsafe { ::std::slice::from_raw_parts(op.z, op.len) };
            // SAFETY: The instruction promises the data is readable for sufficient elements.
            let x = unsafe { ::std::slice::from_raw_parts(op.x, op.len) };

            let basis = local_to_global_pauli(z, x, qubits, num_qubits);
            PbcOperation::Measurement {
                basis,
                flip_result: op.flip_outcome,
            }
        } else {
            // We checked that only PBC operations exist.
            unreachable!("Non PBC operation encountered.")
        }
    });

    Ok(Box::new(iter))
}
