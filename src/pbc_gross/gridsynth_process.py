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
