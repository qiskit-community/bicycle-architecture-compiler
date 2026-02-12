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

mod architecture;
mod basis_changer;
mod compile;
pub mod language;
pub mod operation;
pub mod optimize;
mod small_angle;

pub use architecture::PathArchitecture;

use std::{io::ErrorKind, path::Path};

/// Ensure that the parent directory of `output_path` exists, creating it
/// (and any intermediate directories) if necessary.
///
/// Returns `Ok(())` if the parent directory exists (or was successfully
/// created) and is writable.  Returns `Err` with a descriptive message
/// otherwise.
///
/// When `output_path` has no parent (e.g. a bare filename such as
/// `"table_gross"`), the function returns `Ok(())` without creating
/// anything, since the file will be written to the current directory.
pub fn ensure_parent_dir(output_path: &Path) -> Result<(), String> {
    match output_path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("Cannot create output directory '{}': {e}", dir.display()))?;

            // Verify the directory is writable by creating and removing a
            // uniquely named temporary file (without clobbering existing files).
            let pid = std::process::id();
            let mut probe = None;
            for attempt in 0_u8..8 {
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let candidate = dir.join(format!(".write_probe_{pid}_{nanos}_{attempt}"));
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&candidate)
                {
                    Ok(_) => {
                        probe = Some(candidate);
                        break;
                    }
                    Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                    Err(e) => {
                        return Err(format!(
                            "Output directory '{}' exists but is not writable: {e}",
                            dir.display()
                        ));
                    }
                }
            }
            let probe = probe.ok_or_else(|| {
                format!(
                    "Output directory '{}' exists but a write probe could not be created",
                    dir.display()
                )
            })?;
            // Best-effort cleanup; ignore errors (e.g. on read-only
            // filesystems `create` above would have already failed).
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        // Empty parent (bare filename) or no parent -- file lives in CWD.
        Some(_) | None => Ok(()),
    }
}

#[cfg(test)]
mod test {

    use std::error::Error;

    use crate::language::{AnglePrecision, PbcOperation};

    use super::*;
    use bicycle_cliffords::{
        native_measurement::NativeMeasurement, MeasurementTableBuilder, TWOGROSS_MEASUREMENT,
    };
    use operation::Operations;

    #[test]
    fn integration_test_rotation() -> Result<(), Box<dyn Error>> {
        let program = r#"[
                                    {
                                        "Rotation": {
                                        "basis": [
                                            "X",
                                            "X",
                                            "I",
                                            "I",
                                            "I",
                                            "I",
                                            "I",
                                            "I",
                                            "I",
                                            "I",
                                            "I",
                                            "Y"
                                        ],
                                        "angle": "0.125"
                                        }
                                    }
                                ]"#;
        let parsed: Vec<PbcOperation> = serde_json::from_str(program)?;
        dbg!(&parsed);
        assert_eq!(1, parsed.len());

        let mut builder =
            MeasurementTableBuilder::new(NativeMeasurement::all(), TWOGROSS_MEASUREMENT);
        builder.build();
        let measurement_table = builder.complete()?;

        let architecture = PathArchitecture { data_blocks: 2 };
        let compiled: Vec<_> = parsed
            .into_iter()
            .flat_map(|op| {
                op.compile(
                    &architecture,
                    &measurement_table,
                    AnglePrecision::lit("1e-16"),
                )
            })
            .collect();
        let ops = Operations(compiled);

        println!("{ops}");

        Ok(())
    }
}

#[cfg(test)]
mod ensure_parent_dir_tests {
    use super::*;
    use std::path::PathBuf;

    /// Helper: create a unique temporary directory for each test to avoid
    /// interference when tests run in parallel.
    fn temp_dir(test_name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("bicycle_test_{test_name}_{}", std::process::id()));
        // Start clean
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn creates_missing_single_directory() {
        let base = temp_dir("single");
        let output = base.join("data").join("table_gross");

        assert!(!base.join("data").exists());
        ensure_parent_dir(&output).unwrap();
        assert!(base.join("data").exists());
        assert!(base.join("data").is_dir());

        // Cleanup
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn creates_missing_nested_directories() {
        let base = temp_dir("nested");
        let output = base.join("a").join("b").join("c").join("table_gross");

        assert!(!base.exists());
        ensure_parent_dir(&output).unwrap();
        assert!(base.join("a").join("b").join("c").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn idempotent_on_existing_directory() {
        let base = temp_dir("idempotent");
        let output = base.join("data").join("table_gross");

        std::fs::create_dir_all(base.join("data")).unwrap();
        // Call twice -- should succeed both times without error.
        ensure_parent_dir(&output).unwrap();
        ensure_parent_dir(&output).unwrap();
        assert!(base.join("data").is_dir());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn bare_filename_does_not_create_anything() {
        // A path with no directory component like "table_gross" should
        // succeed without side effects.
        let result = ensure_parent_dir(Path::new("table_gross"));
        assert!(result.is_ok());
    }

    #[test]
    fn relative_path_creates_directory() {
        let base = temp_dir("relative");
        // Simulate the typical usage: ../data/table_gross
        let output = base.join("data").join("table_gross");

        ensure_parent_dir(&output).unwrap();
        assert!(base.join("data").is_dir());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn probe_file_is_cleaned_up() {
        let base = temp_dir("probe_cleanup");
        let output = base.join("data").join("table_gross");

        ensure_parent_dir(&output).unwrap();

        // No temporary write-probe files should remain after return.
        let leftovers = std::fs::read_dir(base.join("data"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".write_probe_")
            })
            .count();
        assert_eq!(0, leftovers, "write-probe files must be cleaned up");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn directory_with_trailing_slash() {
        let base = temp_dir("trailing");
        let output = base.join("data/");
        // Path with trailing slash has parent = base, file = "data/"
        // This should not panic.
        let _ = ensure_parent_dir(&output);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn does_not_clobber_existing_write_probe_file() {
        let base = temp_dir("existing_probe");
        let data_dir = base.join("data");
        let output = data_dir.join("table_gross");

        std::fs::create_dir_all(&data_dir).unwrap();
        let existing_probe = data_dir.join(".write_probe");
        std::fs::write(&existing_probe, b"keep-me").unwrap();

        ensure_parent_dir(&output).unwrap();

        assert_eq!(
            b"keep-me",
            &std::fs::read(&existing_probe).unwrap()[..],
            "ensure_parent_dir must not overwrite an existing .write_probe file"
        );

        let _ = std::fs::remove_dir_all(&base);
    }
}
