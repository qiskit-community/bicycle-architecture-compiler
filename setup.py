# This code is a Qiskit project.
#
# (C) Copyright IBM 2026.
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at https://www.apache.org/licenses/LICENSE-2.0.
#
# Any modifications or derivative works of this code must retain this
# copyright notice, and modified files need to carry a notice indicating
# that they have been altered from the originals.

"""Setup."""

import os

from setuptools import setup
from setuptools_rust import Binding, RustExtension

# Most of this configuration is managed by `pyproject.toml`.  This only includes the extra bits to
# configure `setuptools-rust`, because we do a little dynamic trick with the debug setting, and we
# also want an explicit `setup.py` file to exist so we can manually call
#
#   python setup.py build_rust --inplace --release
#
# to make optimized Rust components even for editable releases, which would otherwise be quite
# unergonomic to do otherwise.


# If RUST_DEBUG is set, force compiling in debug mode. Else, use the default behavior of whether
# it's an editable installation.
rust_debug = True if os.getenv("RUST_DEBUG") == "1" else None

features = []

setup(
    rust_extensions=[
        RustExtension(
            "bicycle_architecture_compiler.numerics",
            "crates/pyext/Cargo.toml",
            args=("--verbose",),
            binding=Binding.PyO3,
            debug=rust_debug,
            features=features,
        )
    ],
    options={"bdist_wheel": {"py_limited_api": "cp311"}},
)
