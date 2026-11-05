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

use std::io::Write;

use bicycle_cliffords::{
    CompleteMeasurementTable, MeasurementTableBuilder, native_measurement::NativeMeasurement,
};

pub(crate) enum CodeType {
    Gross,
    TwoGross,
}

pub(crate) fn get_measurement_table(
    code: CodeType,
    table_path: Option<&str>,
) -> CompleteMeasurementTable {
    let (code_measurement, id) = match code {
        CodeType::Gross => (bicycle_cliffords::GROSS_MEASUREMENT, "gross"),
        CodeType::TwoGross => (bicycle_cliffords::TWOGROSS_MEASUREMENT, "two-gross"),
    };

    let default_path = format!("../table_{id}.dat");
    let fname = table_path.unwrap_or(&default_path);
    let path = ::std::path::Path::new(fname);
    if path.exists() {
        let bytes = std::fs::read(path).unwrap_or_else(|_| panic!("Failed reading {}", fname));
        return bitcode::deserialize::<CompleteMeasurementTable>(&bytes)
            .expect("Failed deserializing");
    }

    println!(
        "No existing table {} found, building measurements...",
        fname
    );
    let mut builder = MeasurementTableBuilder::new(NativeMeasurement::all(), code_measurement);
    builder.build();
    let table = builder
        .complete()
        .expect("Generating a complete measurement table should succeed");
    write_table(&table, path);
    table
}

fn write_table(table: &CompleteMeasurementTable, path: &::std::path::Path) {
    let mut file = ::std::fs::File::create(path).unwrap();
    let bytes = bitcode::serialize(&table).unwrap();
    file.write_all(&bytes).expect("Failed writing");
}
