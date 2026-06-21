#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

use std::{fs, path::Path};

use exo_dag_db_lab::benchmark::{BenchmarkError, BenchmarkFixture, populate_mvp_evidence_fields};

const FIXTURE_PATH: &str = "crates/exo-dag-db-lab/fixtures/benchmarks/mvp_minimum.json";
const NOTES_PATH: &str = "docs/dagdb/post-mvp/05-mvp-fixture-evidence-population-notes.md";

fn main() -> Result<(), BenchmarkError> {
    run(Path::new(FIXTURE_PATH), Path::new(NOTES_PATH))
}

fn run(fixture_path: &Path, notes_path: &Path) -> Result<(), BenchmarkError> {
    let json = fs::read_to_string(fixture_path).map_err(|error| BenchmarkError::Json {
        reason: error.to_string(),
    })?;
    let mut fixture: BenchmarkFixture =
        serde_json::from_str(&json).map_err(|error| BenchmarkError::Json {
            reason: error.to_string(),
        })?;
    let noted_tasks = populate_mvp_evidence_fields(&mut fixture);
    let mut output =
        serde_json::to_string_pretty(&fixture).map_err(|error| BenchmarkError::Json {
            reason: error.to_string(),
        })?;
    output.push('\n');
    fs::write(fixture_path, output).map_err(|error| BenchmarkError::InvalidFixture {
        reason: error.to_string(),
    })?;
    if !noted_tasks.is_empty() {
        let mut notes = String::from("# MVP Fixture Evidence Population Notes\n\n");
        notes.push_str("## Affected Tasks\n\n");
        for task_id in noted_tasks {
            notes.push_str(&format!("- {task_id}\n"));
        }
        fs::write(notes_path, notes).map_err(|error| BenchmarkError::InvalidFixture {
            reason: error.to_string(),
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
            .join("target/dagdb/populate-test")
            .join(name)
    }

    #[test]
    fn populate_mvp_evidence_fields_bin_writes_fixture_without_notes_when_sources_exist() {
        let fixture_path = temp_path("fixture-ok.json");
        let notes_path = temp_path("notes-ok.md");
        fs::create_dir_all(fixture_path.parent().expect("parent")).expect("mkdir");
        let source = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/benchmarks/mvp_minimum.json"),
        )
        .expect("fixture source");
        fs::write(&fixture_path, source).expect("write fixture");
        let _ = fs::remove_file(&notes_path);

        run(&fixture_path, &notes_path).expect("populate");

        let populated = fs::read_to_string(&fixture_path).expect("read populated");
        assert!(populated.contains("\"expected_citation_ids\""));
        assert!(!notes_path.exists());
        fs::remove_file(fixture_path).expect("remove fixture");
    }

    #[test]
    fn populate_mvp_evidence_fields_bin_writes_notes_for_missing_sources() {
        let fixture_path = temp_path("fixture-notes.json");
        let notes_path = temp_path("notes-notes.md");
        fs::create_dir_all(fixture_path.parent().expect("parent")).expect("mkdir");
        let mut fixture: BenchmarkFixture = serde_json::from_str(
            &fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/benchmarks/mvp_minimum.json"),
            )
            .expect("fixture source"),
        )
        .expect("fixture json");
        fixture.tasks[0].expected_citations.clear();
        let mut json = serde_json::to_string_pretty(&fixture).expect("json");
        json.push('\n');
        fs::write(&fixture_path, json).expect("write fixture");

        run(&fixture_path, &notes_path).expect("populate");

        let notes = fs::read_to_string(&notes_path).expect("notes");
        assert!(notes.contains("- t001"));
        fs::remove_file(fixture_path).expect("remove fixture");
        fs::remove_file(notes_path).expect("remove notes");
    }

    #[test]
    fn populate_mvp_evidence_fields_bin_reports_missing_fixture() {
        let fixture_path = temp_path("fixture-missing.json");
        let notes_path = temp_path("notes-missing.md");
        let _ = fs::remove_file(&fixture_path);

        let error = run(&fixture_path, &notes_path).expect_err("missing fixture fails");
        assert!(matches!(error, BenchmarkError::Json { .. }));
    }

    #[test]
    fn populate_mvp_evidence_fields_bin_reports_invalid_json() {
        let fixture_path = temp_path("fixture-invalid.json");
        let notes_path = temp_path("notes-invalid.md");
        fs::create_dir_all(fixture_path.parent().expect("parent")).expect("mkdir");
        fs::write(&fixture_path, "{").expect("write invalid json");

        let error = run(&fixture_path, &notes_path).expect_err("invalid json fails");
        assert!(matches!(error, BenchmarkError::Json { .. }));
        fs::remove_file(fixture_path).expect("remove fixture");
    }

    #[test]
    fn populate_mvp_evidence_fields_bin_reports_missing_notes_parent() {
        let fixture_path = temp_path("fixture-notes-error.json");
        let notes_path = temp_path("missing-notes-dir/notes.md");
        let _ = fs::remove_dir_all(notes_path.parent().expect("notes parent"));
        fs::create_dir_all(fixture_path.parent().expect("parent")).expect("mkdir");
        let mut fixture: BenchmarkFixture = serde_json::from_str(
            &fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/benchmarks/mvp_minimum.json"),
            )
            .expect("fixture source"),
        )
        .expect("fixture json");
        fixture.tasks[0].allowed_memory_ids.clear();
        let mut json = serde_json::to_string_pretty(&fixture).expect("json");
        json.push('\n');
        fs::write(&fixture_path, json).expect("write fixture");

        let error = run(&fixture_path, &notes_path).expect_err("missing notes parent fails");
        assert!(matches!(error, BenchmarkError::InvalidFixture { .. }));
        fs::remove_file(fixture_path).expect("remove fixture");
    }
}
