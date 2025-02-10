#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Pauli {
    I,
    X,
    Z,
    Y,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BicycleISA {
    SyndromeCycle, // Syndrome cycle
    CSSInitZero,   // Initialize the block in |0>^12
    CSSInitPlus,   // Initialize the block in |+>^12
    DestructiveZ,  // Measure all qubits in Z and infer logical Z measurements
    DestructiveX,  // Measure all qubits in X and infer logical X measurements
    // Automorphism generators with x in {0,...,5} and y in {0,1,2} and x+y>0
    Automorphism { x: u8, y: u8 },

    // Measurements
    // Measure qubits 1 and 7 with specified Paulis, one of which must not be identity
    Measure { p1: Pauli, p7: Pauli },
    // Measure qubits 1 and 7 in a joint operation with another block, one of which must not be identity.
    JointMeasure { p1: Pauli, p7: Pauli },
    // Independently measure qubit 1 and qubit 7 in the X or the Z basis
    ParallelMeasure { p1p7: Pauli },

    // Entanglement between two blocks
    JointBellInit, // Initialize two codes into 12 Bell states via rotating donut method
    JointTransversalCX, // Transversal CX using rotating donut

    // Magic
    InitT,                                // Initialization into 8 physical-noise |T> states
    TGate { basis: Pauli, primed: bool }, // Apply exp(iπ/8 P), with P in {X, X', Z, Z'}
}

#[cfg(test)]
mod tests {
    use super::*;
}
