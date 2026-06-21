use std::{error::Error, fs, io, path::PathBuf};

use exo_dag_db_postgres::postgres::DAGDB_SCHEMA_SQL;

const REQUIRED_OPERATIONAL_EVENTS: &[&str] = &[
    "dagdb_approval_request_submitted",
    "dagdb_approval_granted",
    "dagdb_approval_denied",
    "dagdb_record_accepted",
    "dagdb_import_completed",
    "dagdb_export_completed",
    "dagdb_replay_detected",
    "dagdb_idempotency_conflict",
    "dagdb_rls_tenant_violation",
    "dagdb_signature_failure",
    "dagdb_council_operator_decision",
];

const EXPORT_RECEIPT_SUBJECT_KIND_TABLES: &[&str] =
    &["dagdb_receipts", "dagdb_subject_receipt_heads"];

const EXPORT_OUTBOX_SUBJECT_KIND_TABLES: &[&str] = &["dagdb_dag_outbox"];

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn migrations_allow_required_operational_receipt_event_types() -> TestResult {
    let migrations_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut migrations = String::new();
    for entry in fs::read_dir(&migrations_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("sql") {
            let migration_sql = fs::read_to_string(&path).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!("read migration {}: {err}", path.display()),
                )
            })?;
            migrations.push_str(&migration_sql);
            migrations.push('\n');
        }
    }

    for event_type in REQUIRED_OPERATIONAL_EVENTS {
        assert!(
            migrations.contains(event_type),
            "dagdb_receipts event_type constraint must allow {event_type}"
        );
        assert!(
            DAGDB_SCHEMA_SQL.contains(event_type),
            "fresh DAGDB_SCHEMA_SQL dagdb_receipts event_type constraint must allow {event_type}"
        );
    }

    Ok(())
}

#[test]
fn fresh_schema_allows_export_receipt_subject_kind() -> TestResult {
    let lower = DAGDB_SCHEMA_SQL.to_ascii_lowercase();

    for table in EXPORT_RECEIPT_SUBJECT_KIND_TABLES {
        let table_sql = fresh_schema_table_sql(&lower, table)?;
        assert!(
            table_sql.contains("'export'"),
            "fresh DAGDB_SCHEMA_SQL {table} subject_kind constraint must allow export"
        );
    }

    Ok(())
}

#[test]
fn fresh_schema_allows_export_outbox_subject_kind() -> TestResult {
    let lower = DAGDB_SCHEMA_SQL.to_ascii_lowercase();

    for table in EXPORT_OUTBOX_SUBJECT_KIND_TABLES {
        let table_sql = fresh_schema_table_sql(&lower, table)?;
        assert!(
            table_sql.contains("'export'"),
            "fresh DAGDB_SCHEMA_SQL {table} subject_kind constraint must allow export"
        );
    }

    Ok(())
}

fn fresh_schema_table_sql<'a>(schema_sql: &'a str, table: &str) -> io::Result<&'a str> {
    let table_marker = format!("create table if not exists {table}");
    let Some(start) = schema_sql.find(&table_marker) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("fresh DAGDB_SCHEMA_SQL must create {table}"),
        ));
    };
    let Some((table_sql, _)) = schema_sql[start..].split_once("\n);\n") else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("fresh DAGDB_SCHEMA_SQL must terminate {table} definition"),
        ));
    };

    Ok(table_sql)
}
