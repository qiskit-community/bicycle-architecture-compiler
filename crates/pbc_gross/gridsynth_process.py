## (C) Copyright IBM 2025
##
## This code is licensed under the Apache License, Version 2.0. You may
## obtain a copy of this license in the LICENSE.txt file in the root directory
## of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
##
## Any modifications or derivative works of this code must retain this
## copyright notice, and modified files need to carry a notice indicating
## that they have been altered from the originals.


""" A co-process to the main compilation algorithm that synthesizes small-angle rotations.

This avoids having to repeatedly spawn subprocesses of python -m pygridsynth,
which incurs a startup overhead.
"""

from pygridsynth.gridsynth import gridsynth_gates
import mpmath

while True:
    theta_arg = input()
    epsilon_arg = input()
    theta = mpmath.mpmathify(theta_arg)
    epsilon = mpmath.mpmathify(epsilon_arg)

    gates = gridsynth_gates(theta=theta, epsilon=epsilon)
    print(gates)
