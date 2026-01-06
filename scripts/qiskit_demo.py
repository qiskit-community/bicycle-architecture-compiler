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

"""Yield PBC instructions from a Qiskit circuit.

See qiskit_demo.sh for the script consuming the instructions.
"""

import sys
import json

from qiskit import transpile, QuantumCircuit
from qiskit.circuit.library import PauliEvolutionGate
from qiskit.quantum_info import get_clifford_gate_names, SparseObservable
from qiskit.transpiler.passes import LitinskiTransformation

from qiskit_parser import iter_qiskit_pbc_circuit


def build_evolution_circuit(num_qubits, reps):
    """Build a circuit to compile to Gross code ISA."""
    obs = SparseObservable.from_sparse_list(
        [
            (inter, [i, i + 1], -1)
            for inter in ("XX", "YY", "ZZ")
            for i in range(num_qubits - 1)
        ]
        + [("Z", [i], 0.5) for i in range(num_qubits)],
        num_qubits=num_qubits,
    )
    evo = PauliEvolutionGate(obs, time=1 / reps)

    circuit = QuantumCircuit(num_qubits, num_qubits)
    for _ in range(reps):
        circuit.append(evo, circuit.qubits)

    for i, _ in enumerate(circuit.qubits):
        circuit.measure(i, i)

    return circuit


def compile_pbc(circuit):
    """Compile a Qiskit circuit and yield PBC instructions."""
    basis = ["rz", "t", "tdg"] + get_clifford_gate_names()
    tqc = transpile(circuit, basis_gates=basis)

    lit = LitinskiTransformation(fix_clifford=False)
    pbc = lit(tqc)

    for inst in iter_qiskit_pbc_circuit(pbc):
        print(json.dumps(inst).replace(" ", ""))


# read the number of qubits from the command line (or set to 10 as default)
if len(sys.argv) == 1:
    n = 10
else:
    n = int(sys.argv[1])

evo_circuit = build_evolution_circuit(n, reps=n)  # build the circuit
compile_pbc(evo_circuit)  # yield PBC instruction to be consumed by bicycle compiler
