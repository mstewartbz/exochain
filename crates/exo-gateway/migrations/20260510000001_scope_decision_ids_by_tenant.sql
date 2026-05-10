ALTER TABLE decisions
    DROP CONSTRAINT IF EXISTS decisions_pkey;

ALTER TABLE decisions
    ADD CONSTRAINT decisions_pkey PRIMARY KEY (tenant_id, id_hash);
