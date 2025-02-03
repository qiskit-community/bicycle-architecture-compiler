pub enum Pauli {
    I,
    X,
    Y,
    Z,
}

pub enum BicycleISA {
    SyndromeCycle,                 // Syndrome cycle
    CSSInitZero,                   // Initialize the block in |0>^12
    CSSInitPlus,                   // Initialize the block in |+>^12
    DestructiveZ,                  // Measure all qubits in Z and infer logical Z measurements
    DestructiveX,                  // Meaasure all qubits in X and infer logical X measurements
    Automorphism { x: u8, y: u8 }, // Automorphism generators.

    // Measurements
    Measure { p1: Pauli, p7: Pauli }, // Measure qubits 1 and 7 with specified Paulis
    JointMeasure { p1: Pauli, p7: Pauli }, // Measure qubits 1 and 7 in a joint operation with another block
    ParallelMeasure { p1p7: Pauli }, // Independently measure qubit 1 and qubit 7 in the same basis simultaneously.

    // Entanglement between two blocks
    JointBellInit, // Initialize two codes into 12 Bell states via rotating donut method
    JointTransversalCX, // Transversal CX using rotating donut

    // Magic
    InitT,                             // Initialization into 8 physical-noise |T> states
    TGate { basis: Pauli, qubit: u8 }, // Specify either X, X', Z, or Z'.
}

#[cfg(test)]
mod tests {
    use super::*;
}
