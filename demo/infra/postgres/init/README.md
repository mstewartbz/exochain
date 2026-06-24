# Legacy Demo SQL Fixtures

These SQL files are fixture-only rollback and schema-reference artifacts for
tests and historical demo data inspection.

They must not be mounted as a production writer for EXOCHAIN demo services. The
runtime services use the shared demo DAG DB adapter and write through the
configured EXOCHAIN DAG DB gateway.
