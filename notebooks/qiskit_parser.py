# Copyright contributors to the Bicycle Architecture Compiler project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Qiskit circuit parser for bicycle compiler."""

from __future__ import annotations
from collections.abc import Iterator
import json
import numpy as np

PAULI_TABLE = {
    (True, True): "Y",
    (True, False): "Z",
    (False, True): "X",
    (False, False): "I",
}


def iter_qiskit_pbc_circuit(
    pbc: "QuantumCircuit", as_str: bool = False
) -> Iterator[dict] | Iterator[str]:
    """Yield PBC instructions consumable by the bicycle compiler.

    Args:
        pbc: The Qiskit ``QuantumCircuit`` object to iterate over. This circuit is required to
            be in PBC format, i.e. contain only ``PauliEvolutionGate`` objects with a single
            Pauli as operator, and ``PauliProductMeasurement`` instructions.
        as_str: If ``True``, yield instructions as string that's directly consumable by
            the ``bicycle_compiler`` executable. If ``False``, return the plain dictionary.

    Returns:
        An iterator over PBC instructions in the bicycle compilers JSON format, that is
        ``{"Rotation": {"basis": ["Z", "X", "Y", "I"], "angle": 0.123}}`` or
        ``{"Measurement": {"basis": ["Z", "X", "Y", "I"], "flipped": True}}``.
        If ``as_str`` is ``True``, the dictionaries are JSON serialized and whitespaces removed.

    Raises:
        ValueError: If the input cirucit is not in the required PBC format.
    """

    qubit_to_index = {qubit: index for index, qubit in enumerate(pbc.qubits)}

    # potentially transform the instruction to string format
    if as_str:
        to_str = lambda inst: json.dumps(inst).replace(" ", "")
    else:
        to_str = lambda inst: inst  # no op

    for i, inst in enumerate(pbc.data):
        if inst.name == "PauliEvolution":
            evo = inst.operation
            if isinstance(evo.operator, list):
                raise ValueError("Grouped operators in Pauli not supported.")

            op = evo.operator.to_sparse_list()
            if len(op) > 1:
                raise ValueError("PauliEvolution is not a single rotation.")
            paulis, indices, coeff = op[0]

            basis = ["I"] * pbc.num_qubits
            for pauli, i in zip(paulis, indices):
                basis[i] = pauli

            angle = evo.params[0] * np.real(coeff)

            rot = {"Rotation": {"basis": basis, "angle": str(angle)}}
            yield to_str(rot)

        elif inst.name == "pauli_product_measurement":
            ppm = inst.operation

            # TODO Use a public interface, once available.
            # See also https://github.com/Qiskit/qiskit/issues/15468.
            z, x, phase = ppm._to_pauli_data()

            basis = ["I"] * pbc.num_qubits
            for qubit, zq, xq in zip(inst.qubits, z, x):
                basis[qubit_to_index[qubit]] = PAULI_TABLE[(zq, xq)]

            flipped = bool(phase == 2)
            meas = {"Measurement": {"basis": basis, "flip_result": flipped}}
            yield to_str(meas)

        else:
            raise ValueError(f"Unsupported instruction in PBC circuit: {inst.name}")
