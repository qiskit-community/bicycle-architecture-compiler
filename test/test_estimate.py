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

import os
import unittest
from typing import cast

from qiskit.circuit import QuantumCircuit, Parameter
from qiskit.circuit.library import PauliEvolutionGate
from qiskit.transpiler import generate_preset_pbc_pass_manager
from qiskit.quantum_info import SparseObservable

from bicycle_architecture_compiler import estimate, estimate_from_config


ERROR_MODELS = os.path.join(os.path.dirname(__file__), "error_models")


class TestEstimateFunction(unittest.TestCase):
    def setUp(self):
        super().setUp()
        self.pm = generate_preset_pbc_pass_manager()

    def test_estimate(self):
        bell = QuantumCircuit(2)
        bell.h(0)
        bell.cx(0, 1)

        data = estimate(self.pm.run(bell), "gross", "1e-3", "1e-10")
        self.assertGreater(data["total_error"], 0.0)
        self.assertGreater(data["end_time"], 0.0)

        counts = data["counts"]
        self.assertGreater(counts["automorphisms"], 0)
        self.assertGreater(counts["measurements"], 0)

        self.assertEqual(counts["t_injs"], 0)
        self.assertEqual(counts["joint_measurements"], 0)

    def test_model_config(self):
        bell = QuantumCircuit(2)
        bell.h(0)
        bell.cx(0, 1)
        pbc = self.pm.run(bell)

        # Same values as the built-in gross 1e-3 preset
        path = os.path.join(ERROR_MODELS, "gross3.toml")
        custom = estimate_from_config(pbc, "gross", path, "1e-10")
        self.assertEqual(custom, estimate(pbc, "gross", "1e-3", "1e-10"))

    def test_missing_model_config(self):
        path = os.path.join(ERROR_MODELS, "does_not_exist.toml")
        with self.assertRaises(ValueError):
            estimate_from_config(QuantumCircuit(), "gross", path, "1e-10")

    def test_zero_idle_model_config(self):
        path = os.path.join(ERROR_MODELS, "zero_idle.toml")
        with self.assertRaisesRegex(ValueError, "timing.idle must be non-zero"):
            estimate_from_config(QuantumCircuit(), "gross", path, "1e-10")

    def test_zero_model_config(self):
        bell = QuantumCircuit(2)
        bell.h(0)
        bell.cx(0, 1)

        path = os.path.join(ERROR_MODELS, "zero.toml")
        data = estimate_from_config(self.pm.run(bell), "gross", path, "1e-10")
        self.assertEqual(data["total_error"], 0.0)
        self.assertEqual(data["end_time"], 0)

        # The circuit still compiles to instructions, they are just free
        counts = data["counts"]
        self.assertGreater(counts["automorphisms"], 0)
        self.assertGreater(counts["measurements"], 0)

    def test_noncircuit(self):
        with self.assertRaises(TypeError):
            estimate("banana", "gross", "1e-3", "1e-10")

        with self.assertRaises(TypeError):
            estimate(QuantumCircuit(10).to_dag(), "gross", "1e-3", "1e-10")

    def test_parameterized_circuit(self):
        qc = QuantumCircuit(1)
        qc.h(0)
        qc.p(Parameter("i do not belong here"), 0)
        qc.x(0)

        pbc = self.pm.run(qc)
        with self.assertRaises(TypeError):
            estimate(pbc, "gross", "1e-3", "1e-10")

    def test_non_pbc_circuit(self):
        bell = QuantumCircuit(2)
        bell.h(0)
        bell.cx(0, 1)

        with self.assertRaises(TypeError):
            estimate(bell, "gross", "1e-3", "1e-10")

    def test_empty(self):
        data = estimate(QuantumCircuit(), "gross", "1e-3", "1e-10")
        self.assertEqual(data["total_error"], 0.0)
        self.assertEqual(data["end_time"], 0.0)
        self.assertEqual(
            data["counts"],
            {
                "automorphisms": 0,
                "t_injs": 0,
                "joint_measurements": 0,
                "measurements": 0,
                "idles": 0,
            },
        )

    def test_integration(self):
        """Build a circuit to compile to Gross code ISA."""
        num_qubits = 100
        obs = SparseObservable.from_sparse_list(
            [("Z", [i], 0.5) for i in range(num_qubits)]
            + [
                (inter, [i, i + 1], -1)
                for i in range(num_qubits - 1)
                for inter in ("XX", "YY", "ZZ")
            ],
            num_qubits=num_qubits,
        )

        reps = 100
        evo = PauliEvolutionGate(obs, time=1 / reps).definition
        evo = cast(QuantumCircuit, evo)  # type checkers need hand holding sometimes

        circuit = QuantumCircuit(num_qubits, num_qubits)
        for _ in range(reps):
            circuit.compose(evo, circuit.qubits, inplace=True)

        for i, _ in enumerate(circuit.qubits):
            circuit.measure(i, i)

        pbc = self.pm.run(circuit)

        errs = {
            "gross3": estimate(pbc, "gross", "1e-3", "1e-10")["total_error"],
            "gross4": estimate(pbc, "gross", "1e-4", "1e-10")["total_error"],
            "twogross3": estimate(pbc, "two_gross", "1e-3", "1e-10")["total_error"],
            "twogross4": estimate(pbc, "two_gross", "1e-4", "1e-10")["total_error"],
        }

        self.assertGreater(errs["twogross4"], 0)
        self.assertGreater(errs["twogross3"], errs["twogross4"])
        self.assertGreater(errs["gross4"], errs["twogross3"])
        self.assertGreater(errs["gross3"], errs["gross4"])
