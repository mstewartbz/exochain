ALTER TABLE dagdb_dag_outbox DROP CONSTRAINT IF EXISTS dagdb_dag_outbox_subject_kind_check;
ALTER TABLE dagdb_dag_outbox
    ADD CONSTRAINT dagdb_dag_outbox_subject_kind_check
    CHECK (subject_kind IN ('memory','catalog','route','context_packet','validation_report','agent_safety_score','inbound_agent_credential','council_decision','export'));
