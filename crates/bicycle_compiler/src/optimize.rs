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

use bicycle_common::{AutomorphismData, BicycleISA};

use crate::operation::Operation;

/// Remove measurements that are repeated on the same block
/// Note: This considers only single-block measurements for simplicity
pub fn remove_duplicate_measurements(
    ops: impl IntoIterator<Item = Operation>,
) -> impl Iterator<Item = Operation> {
    remove_duplicate_measurements_chunked(ops.into_iter().map(|op| vec![op])).flatten()
}

/// Remove measurements that are repeated but respect the chunk boundaries as they are given
pub fn remove_duplicate_measurements_chunked(
    chunked_ops: impl IntoIterator<Item = impl IntoIterator<Item = Operation>>,
) -> impl Iterator<Item = Vec<Operation>> {
    let mut history: Vec<Option<BicycleISA>> = Vec::new();

    chunked_ops.into_iter().map(move |ops_chunk| {
        ops_chunk
            .into_iter()
            .filter(|ops_list| {
                for (i, instr) in ops_list {
                    history.resize_with(history.len().max(i + 1), Default::default);

                    if let BicycleISA::Measure(_) = instr {
                        if history[*i] == Some(*instr) {
                            return false;
                        }
                    }
                    // Copy seen instructions into history
                    // Cannot reference because that would make instructions immutable
                    history[*i] = Some(*instr);
                }
                true
            })
            .collect()
    })
}

/// Remove automorphisms that apply a zero shift
pub fn remove_trivial_automorphisms(
    ops: impl IntoIterator<Item = Operation>,
) -> impl Iterator<Item = Operation> {
    ops.into_iter().filter(|op| match op[..] {
        [(_, BicycleISA::Automorphism(autdata))] => autdata != AutomorphismData::new(0, 0),
        _ => true,
    })
}

#[cfg(test)]
mod tests {
    use bicycle_common::TwoBases;

    use super::*;
    use bicycle_common::Pauli::{X, Y, Z};

    #[test]
    fn remove_duplicate_meas() {
        let meas = BicycleISA::Measure(TwoBases::new(X, Z).unwrap());
        let ops = vec![vec![(3, meas)], vec![(3, meas)]];

        let res: Vec<_> = remove_duplicate_measurements(ops).collect();
        let expected = vec![vec![(3, meas)]];
        assert_eq!(expected, res);
    }

    #[test]
    fn remove_duplicat_meas2() {
        let meas = BicycleISA::Measure(TwoBases::new(X, Z).unwrap());
        let ops = vec![vec![(3, meas)], vec![(0, meas)], vec![(3, meas)]];

        let res: Vec<_> = remove_duplicate_measurements(ops).collect();
        let expected = vec![vec![(3, meas)], vec![(0, meas)]];
        assert_eq!(expected, res);
    }

    #[test]
    fn remove_trivial_auts() {
        let nontrivial_aut = BicycleISA::Automorphism(AutomorphismData::new(3, 4));
        let trivial_aut = BicycleISA::Automorphism(AutomorphismData::new(0, 0));
        let measurement = BicycleISA::Measure(TwoBases::new(X, Y).unwrap());
        let ops = vec![
            vec![(5, nontrivial_aut)],
            vec![(2, trivial_aut)],
            vec![(10, measurement)],
            vec![(0, nontrivial_aut)],
            vec![(0, trivial_aut)],
        ];

        let res: Vec<_> = remove_trivial_automorphisms(ops).collect();

        assert_eq!(
            res,
            vec![
                vec![(5, nontrivial_aut)],
                vec![(10, measurement)],
                vec![(0, nontrivial_aut)]
            ]
        );
    }
}
