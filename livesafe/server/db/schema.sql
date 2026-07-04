-- LiveSafe.ai Database Schema
-- PostgreSQL schema for off-chain operational data

-- Subscribers (core subscriber accounts)
CREATE TABLE IF NOT EXISTS subscribers (
  id SERIAL PRIMARY KEY,
  did VARCHAR(255) UNIQUE NOT NULL,
  email VARCHAR(255) UNIQUE NOT NULL,
  password_hash VARCHAR(255) NOT NULL,
  email_verified BOOLEAN DEFAULT FALSE,
  verification_token VARCHAR(255),
  first_name VARCHAR(100),
  last_name VARCHAR(100),
  date_of_birth DATE,
  blood_type VARCHAR(5),
  dnr_status VARCHAR(50) DEFAULT 'not_specified',
  organ_donor BOOLEAN DEFAULT FALSE,
  role VARCHAR(50) DEFAULT 'subscriber',
  is_hero BOOLEAN DEFAULT FALSE,
  is_military BOOLEAN DEFAULT FALSE,
  alert_sensitivity VARCHAR(50) DEFAULT 'always',
  phone VARCHAR(50),
  phone_verified BOOLEAN DEFAULT FALSE,
  phone_verification_code VARCHAR(10),
  phone_verification_expires_at TIMESTAMP WITH TIME ZONE,
  consent_default_scope VARCHAR(50) DEFAULT 'basic_health',
  consent_default_duration_days INTEGER DEFAULT 30,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Add hero/free-tier flags for existing subscriber tables
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='subscribers' AND column_name='is_hero') THEN
    ALTER TABLE subscribers ADD COLUMN is_hero BOOLEAN DEFAULT FALSE;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='subscribers' AND column_name='is_military') THEN
    ALTER TABLE subscribers ADD COLUMN is_military BOOLEAN DEFAULT FALSE;
  END IF;
END $$;

-- Subscriber medical data
CREATE TABLE IF NOT EXISTS subscriber_allergies (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  allergy VARCHAR(255) NOT NULL,
  severity VARCHAR(50),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS subscriber_medications (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  medication VARCHAR(255) NOT NULL,
  dosage VARCHAR(100),
  frequency VARCHAR(100),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS subscriber_conditions (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  condition_name VARCHAR(255) NOT NULL,
  diagnosed_date DATE,
  notes TEXT,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS emergency_contacts (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  name VARCHAR(255) NOT NULL,
  phone VARCHAR(50) NOT NULL,
  relationship VARCHAR(100),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Trustees (PACE system)
CREATE TABLE IF NOT EXISTS trustees (
  id SERIAL PRIMARY KEY,
  did VARCHAR(255) UNIQUE,
  email VARCHAR(255) NOT NULL,
  password_hash VARCHAR(255),
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  role VARCHAR(50) NOT NULL,
  status VARCHAR(50) DEFAULT 'pending',
  shard_ref VARCHAR(255),
  invitation_token VARCHAR(255),
  invitation_expires_at TIMESTAMP WITH TIME ZONE,
  accepted_at TIMESTAMP WITH TIME ZONE,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- First responders
CREATE TABLE IF NOT EXISTS responders (
  id SERIAL PRIMARY KEY,
  did VARCHAR(255) UNIQUE NOT NULL,
  email VARCHAR(255) UNIQUE NOT NULL,
  password_hash VARCHAR(255) NOT NULL,
  agency_id INTEGER,
  role VARCHAR(100),
  certification VARCHAR(255),
  is_military BOOLEAN DEFAULT FALSE,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Healthcare providers
CREATE TABLE IF NOT EXISTS providers (
  id SERIAL PRIMARY KEY,
  did VARCHAR(255) UNIQUE NOT NULL,
  email VARCHAR(255) UNIQUE NOT NULL,
  password_hash VARCHAR(255) NOT NULL,
  npi VARCHAR(10) UNIQUE,
  facility VARCHAR(255),
  specialty VARCHAR(255),
  verified BOOLEAN DEFAULT FALSE,
  npi_verified BOOLEAN DEFAULT FALSE,
  provider_name VARCHAR(255),
  npi_taxonomy VARCHAR(255),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Agencies (fire, EMS, hospital, police)
CREATE TABLE IF NOT EXISTS agencies (
  id SERIAL PRIMARY KEY,
  name VARCHAR(255) NOT NULL,
  type VARCHAR(100) NOT NULL,
  admin_email VARCHAR(255),
  verified BOOLEAN DEFAULT FALSE,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Add agency foreign key to responders (skip if already exists)
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE constraint_name = 'fk_responders_agency'
    AND table_name = 'responders'
  ) THEN
    ALTER TABLE responders
      ADD CONSTRAINT fk_responders_agency
      FOREIGN KEY (agency_id) REFERENCES agencies(id)
      ON DELETE SET NULL;
  END IF;
END $$;

-- Cards (QR/NFC)
CREATE TABLE IF NOT EXISTS cards (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  qr_data TEXT NOT NULL,
  nfc_payload TEXT,
  emergency_consent_token VARCHAR(255),
  status VARCHAR(50) DEFAULT 'active',
  issued_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  expires_at TIMESTAMP WITH TIME ZONE
);

-- Add emergency_consent_token column if it doesn't exist (migration for existing installs)
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='cards' AND column_name='emergency_consent_token') THEN
    ALTER TABLE cards ADD COLUMN emergency_consent_token VARCHAR(255);
  END IF;
END $$;

-- Scan events
CREATE TABLE IF NOT EXISTS scans (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id),
  responder_id INTEGER REFERENCES responders(id),
  card_id INTEGER REFERENCES cards(id),
  scan_type VARCHAR(50) DEFAULT 'emergency',
  location_lat DECIMAL(10, 7),
  location_lng DECIMAL(10, 7),
  access_token VARCHAR(255),
  access_expires_at TIMESTAMP WITH TIME ZONE,
  flagged_for_followup BOOLEAN DEFAULT FALSE,
  followup_notes TEXT,
  scanned_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Medical records (bailment)
CREATE TABLE IF NOT EXISTS medical_records (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  title VARCHAR(255) NOT NULL,
  record_type VARCHAR(100),
  file_path VARCHAR(500),
  file_format VARCHAR(50),
  encrypted BOOLEAN DEFAULT TRUE,
  visibility VARCHAR(50) DEFAULT 'private',
  version INTEGER DEFAULT 1,
  parent_record_id INTEGER REFERENCES medical_records(id),
  category VARCHAR(100),
  annotation TEXT,
  exochain_receipt VARCHAR(255),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Record requests (HIPAA Right of Access)
CREATE TABLE IF NOT EXISTS record_requests (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  provider_name VARCHAR(255) NOT NULL,
  provider_npi VARCHAR(10),
  provider_address TEXT,
  status VARCHAR(50) DEFAULT 'sent',
  letter_pdf_path VARCHAR(500),
  sent_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  received_at TIMESTAMP WITH TIME ZONE
);

-- Provider consent events
CREATE TABLE IF NOT EXISTS consent_events (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  provider_id INTEGER REFERENCES providers(id),
  scope TEXT NOT NULL,
  purpose VARCHAR(255),
  granted_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  expires_at TIMESTAMP WITH TIME ZONE,
  revoked_at TIMESTAMP WITH TIME ZONE,
  exochain_receipt VARCHAR(255)
);

-- Credentials vault
CREATE TABLE IF NOT EXISTS credentials (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  credential_type VARCHAR(100) NOT NULL,
  title VARCHAR(255),
  data_encrypted TEXT,
  carrier VARCHAR(255),
  member_id VARCHAR(100),
  group_number VARCHAR(100),
  effective_date DATE,
  expiry_date DATE,
  visibility VARCHAR(50) DEFAULT 'private',
  exochain_receipt TEXT,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Notifications
CREATE TABLE IF NOT EXISTS notifications (
  id SERIAL PRIMARY KEY,
  recipient_did VARCHAR(255) NOT NULL,
  recipient_type VARCHAR(50),
  channel VARCHAR(50) NOT NULL,
  notification_type VARCHAR(100),
  title VARCHAR(255),
  body TEXT,
  status VARCHAR(50) DEFAULT 'pending',
  read BOOLEAN DEFAULT FALSE,
  sent_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Audit receipts
CREATE TABLE IF NOT EXISTS audit_receipts (
  id SERIAL PRIMARY KEY,
  subject_did VARCHAR(255) NOT NULL,
  actor_did VARCHAR(255),
  event_type VARCHAR(100) NOT NULL,
  scope VARCHAR(255),
  details JSONB,
  receipt_hash VARCHAR(255),
  previous_hash VARCHAR(255),
  exochain_receipt VARCHAR(255),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Governance workflows (recovery, trustee replacement, emergency access)
CREATE TABLE IF NOT EXISTS governance_workflows (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id),
  workflow_type VARCHAR(100) NOT NULL,
  required_signers INTEGER NOT NULL,
  current_signers INTEGER DEFAULT 0,
  signers JSONB DEFAULT '[]',
  deadline_at TIMESTAMP WITH TIME ZONE,
  status VARCHAR(50) DEFAULT 'pending',
  result VARCHAR(100),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  completed_at TIMESTAMP WITH TIME ZONE
);

-- 0dentity scoring tables
CREATE TABLE IF NOT EXISTS odentity_scores (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  dimension VARCHAR(100) NOT NULL,
  current_score DECIMAL(5, 2) DEFAULT 0,
  max_possible DECIMAL(5, 2) DEFAULT 100,
  claim_count INTEGER DEFAULT 0,
  last_updated TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  UNIQUE(subscriber_id, dimension)
);

CREATE TABLE IF NOT EXISTS odentity_claims (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  claim_type VARCHAR(100) NOT NULL,
  dimension VARCHAR(100) NOT NULL,
  points_awarded DECIMAL(5, 2) NOT NULL,
  issuer VARCHAR(255),
  credential_hash VARCHAR(255),
  issued_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  expires_at TIMESTAMP WITH TIME ZONE,
  revoked_at TIMESTAMP WITH TIME ZONE,
  exochain_receipt VARCHAR(255)
);

CREATE TABLE IF NOT EXISTS odentity_trust_events (
  id SERIAL PRIMARY KEY,
  event_type VARCHAR(100) NOT NULL,
  actor_subscriber_id INTEGER REFERENCES subscribers(id),
  target_subscriber_id INTEGER REFERENCES subscribers(id),
  dimension VARCHAR(100) NOT NULL,
  delta_points DECIMAL(5, 2) NOT NULL,
  occurred_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  exochain_receipt VARCHAR(255)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_subscribers_did ON subscribers(did);
CREATE INDEX IF NOT EXISTS idx_subscribers_email ON subscribers(email);
CREATE INDEX IF NOT EXISTS idx_trustees_subscriber ON trustees(subscriber_id);
CREATE INDEX IF NOT EXISTS idx_scans_subscriber ON scans(subscriber_id);
CREATE INDEX IF NOT EXISTS idx_scans_responder ON scans(responder_id);
CREATE INDEX IF NOT EXISTS idx_medical_records_subscriber ON medical_records(subscriber_id);
CREATE INDEX IF NOT EXISTS idx_consent_events_subscriber ON consent_events(subscriber_id);
CREATE INDEX IF NOT EXISTS idx_consent_events_provider ON consent_events(provider_id);
CREATE INDEX IF NOT EXISTS idx_notifications_recipient ON notifications(recipient_did);
CREATE INDEX IF NOT EXISTS idx_audit_receipts_subject ON audit_receipts(subject_did);
CREATE INDEX IF NOT EXISTS idx_odentity_scores_subscriber ON odentity_scores(subscriber_id);
CREATE INDEX IF NOT EXISTS idx_odentity_claims_subscriber ON odentity_claims(subscriber_id);
CREATE INDEX IF NOT EXISTS idx_credentials_subscriber ON credentials(subscriber_id);
CREATE INDEX IF NOT EXISTS idx_trustees_email ON trustees(email);

-- P.A.C.E. canonical role migration and invitation delivery tracking
DO $$ BEGIN
  UPDATE trustees SET role = 'contingent' WHERE role = 'custodial';

  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'invite_phone') THEN
    ALTER TABLE trustees ADD COLUMN invite_phone VARCHAR(50);
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'invitation_url') THEN
    ALTER TABLE trustees ADD COLUMN invitation_url TEXT;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'delivery_channel') THEN
    ALTER TABLE trustees ADD COLUMN delivery_channel VARCHAR(50) DEFAULT 'link';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'email_delivery_status') THEN
    ALTER TABLE trustees ADD COLUMN email_delivery_status VARCHAR(50) DEFAULT 'not_requested';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'sms_delivery_status') THEN
    ALTER TABLE trustees ADD COLUMN sms_delivery_status VARCHAR(50) DEFAULT 'not_requested';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'last_sent_at') THEN
    ALTER TABLE trustees ADD COLUMN last_sent_at TIMESTAMP WITH TIME ZONE;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'delivery_error_code') THEN
    ALTER TABLE trustees ADD COLUMN delivery_error_code VARCHAR(100);
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'declined_at') THEN
    ALTER TABLE trustees ADD COLUMN declined_at TIMESTAMP WITH TIME ZONE;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'trustees_pace_role_check') THEN
    ALTER TABLE trustees
      ADD CONSTRAINT trustees_pace_role_check
      CHECK (role IN ('primary', 'alternate', 'contingent', 'emergency')) NOT VALID;
  END IF;
END $$;

-- Migrations: add first_name, last_name to trustees (for trustee accounts)
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'first_name') THEN
    ALTER TABLE trustees ADD COLUMN first_name VARCHAR(100);
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'last_name') THEN
    ALTER TABLE trustees ADD COLUMN last_name VARCHAR(100);
  END IF;
END $$;

-- Remove UNIQUE constraint on trustees.did to allow same trustee serving multiple subscribers
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'trustees_did_key') THEN
    ALTER TABLE trustees DROP CONSTRAINT trustees_did_key;
  END IF;
END $$;

-- VSS key shard ceremonies (tracks when all 4 PACE trustees accept and VSS generation is triggered)
CREATE TABLE IF NOT EXISTS vss_ceremonies (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  ceremony_type VARCHAR(50) NOT NULL DEFAULT 'initial',
  threshold INTEGER NOT NULL DEFAULT 3,
  total_shares INTEGER NOT NULL DEFAULT 4,
  master_key_hash VARCHAR(255) NOT NULL,
  status VARCHAR(50) DEFAULT 'completed',
  triggered_by VARCHAR(100),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_vss_ceremonies_subscriber ON vss_ceremonies(subscriber_id);

-- Add VSS-specific columns to trustees
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'shard_encrypted') THEN
    ALTER TABLE trustees ADD COLUMN shard_encrypted TEXT;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'shard_index') THEN
    ALTER TABLE trustees ADD COLUMN shard_index INTEGER;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'vss_ceremony_id') THEN
    ALTER TABLE trustees ADD COLUMN vss_ceremony_id INTEGER;
  END IF;
END $$;

-- Add metadata column to governance_workflows for workflow-specific data
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'governance_workflows' AND column_name = 'metadata') THEN
    ALTER TABLE governance_workflows ADD COLUMN metadata JSONB DEFAULT '{}';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'governance_workflows' AND column_name = 'updated_at') THEN
    ALTER TABLE governance_workflows ADD COLUMN updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW();
  END IF;
END $$;

-- Add replaced_at and replacement_workflow_id columns to trustees
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'replaced_at') THEN
    ALTER TABLE trustees ADD COLUMN replaced_at TIMESTAMP WITH TIME ZONE;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'trustees' AND column_name = 'replacement_workflow_id') THEN
    ALTER TABLE trustees ADD COLUMN replacement_workflow_id INTEGER;
  END IF;
END $$;

-- Identity recovery workflows table
CREATE TABLE IF NOT EXISTS identity_recovery_workflows (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  governance_workflow_id INTEGER REFERENCES governance_workflows(id),
  initiated_by VARCHAR(255),
  status VARCHAR(50) DEFAULT 'pending',
  quorum_met BOOLEAN DEFAULT FALSE,
  recovery_event_id INTEGER,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  completed_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS idx_identity_recovery_subscriber ON identity_recovery_workflows(subscriber_id);

-- Research bridge: clinical trial matching opt-in (CyberMedica integration)
CREATE TABLE IF NOT EXISTS research_opt_ins (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  subscriber_did VARCHAR(255) NOT NULL,
  opted_in BOOLEAN DEFAULT FALSE,
  opt_in_at TIMESTAMP WITH TIME ZONE,
  opt_out_at TIMESTAMP WITH TIME ZONE,
  consent_scope VARCHAR(100) DEFAULT 'de_identified_trial_matching',
  cybermedica_consent_ref VARCHAR(255),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  UNIQUE(subscriber_id)
);

CREATE INDEX IF NOT EXISTS idx_research_opt_ins_subscriber ON research_opt_ins(subscriber_id);
CREATE INDEX IF NOT EXISTS idx_research_opt_ins_did ON research_opt_ins(subscriber_did);

-- Add missing columns to medical_records table
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'medical_records' AND column_name = 'extracted_data') THEN
    ALTER TABLE medical_records ADD COLUMN extracted_data JSONB;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'medical_records' AND column_name = 'file_size') THEN
    ALTER TABLE medical_records ADD COLUMN file_size BIGINT;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'medical_records' AND column_name = 'visibility_providers') THEN
    ALTER TABLE medical_records ADD COLUMN visibility_providers JSONB;
  END IF;
END $$;

-- Device signing keys (multi-device support with revocation)
CREATE TABLE IF NOT EXISTS device_signing_keys (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  device_id VARCHAR(255) NOT NULL,
  key_ref VARCHAR(255) UNIQUE NOT NULL,
  public_key TEXT NOT NULL,
  device_name VARCHAR(255),
  is_active BOOLEAN DEFAULT TRUE,
  revoked_at TIMESTAMP WITH TIME ZONE,
  revoked_reason VARCHAR(255),
  revoked_by INTEGER,
  last_used_at TIMESTAMP WITH TIME ZONE,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  UNIQUE(subscriber_id, device_id)
);

CREATE INDEX IF NOT EXISTS idx_device_signing_keys_subscriber ON device_signing_keys(subscriber_id);
CREATE INDEX IF NOT EXISTS idx_device_signing_keys_key_ref ON device_signing_keys(key_ref);

-- Marketplace catalog, Ambientli imported evidence, and launch library
CREATE TABLE IF NOT EXISTS marketplace_import_batches (
  id SERIAL PRIMARY KEY,
  source_label VARCHAR(100) NOT NULL,
  zip_sha256 VARCHAR(64) NOT NULL,
  export_sha256 VARCHAR(64) NOT NULL,
  exported_at TIMESTAMP WITH TIME ZONE,
  entity_counts JSONB NOT NULL DEFAULT '{}',
  field_inventory JSONB NOT NULL DEFAULT '{}',
  import_mode VARCHAR(50) DEFAULT 'dry_run' NOT NULL,
  review_status VARCHAR(50) DEFAULT 'pending' NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  UNIQUE(source_label, export_sha256)
);

CREATE TABLE IF NOT EXISTS marketplace_import_records (
  id SERIAL PRIMARY KEY,
  batch_id INTEGER REFERENCES marketplace_import_batches(id) ON DELETE CASCADE,
  source_system VARCHAR(100) NOT NULL,
  source_entity VARCHAR(100) NOT NULL,
  source_id VARCHAR(255),
  source_sha256 VARCHAR(64) NOT NULL,
  review_status VARCHAR(50) DEFAULT 'pending' NOT NULL,
  safe_excerpt JSONB NOT NULL DEFAULT '{}',
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  UNIQUE(source_system, source_entity, source_id, source_sha256)
);

CREATE TABLE IF NOT EXISTS marketplace_catalog_items (
  id SERIAL PRIMARY KEY,
  import_batch_id INTEGER REFERENCES marketplace_import_batches(id) ON DELETE SET NULL,
  source_record_id INTEGER REFERENCES marketplace_import_records(id) ON DELETE SET NULL,
  source_system VARCHAR(100) NOT NULL,
  source_id VARCHAR(255),
  source_sha256 VARCHAR(64) NOT NULL,
  slug VARCHAR(255) UNIQUE NOT NULL,
  object_type VARCHAR(50) NOT NULL,
  category VARCHAR(100) NOT NULL,
  title VARCHAR(255) NOT NULL,
  summary TEXT NOT NULL,
  icon VARCHAR(100),
  tags TEXT[] DEFAULT '{}',
  content_json JSONB NOT NULL,
  plan_gate VARCHAR(100) NOT NULL,
  consent_requirement VARCHAR(100) NOT NULL,
  audit_behavior VARCHAR(100) NOT NULL,
  disablement_behavior VARCHAR(100) NOT NULL,
  visibility VARCHAR(50) DEFAULT 'public' NOT NULL,
  launch_status VARCHAR(50) DEFAULT 'draft' NOT NULL,
  review_status VARCHAR(50) DEFAULT 'pending' NOT NULL,
  contains_sensitive_info BOOLEAN DEFAULT FALSE NOT NULL,
  public_claims_allowed BOOLEAN DEFAULT FALSE NOT NULL,
  historical_install_count INTEGER DEFAULT 0 NOT NULL,
  historical_rating_average DECIMAL(3, 2) DEFAULT 0 NOT NULL,
  historical_rating_count INTEGER DEFAULT 0 NOT NULL,
  historical_report_count INTEGER DEFAULT 0 NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  CHECK (public_claims_allowed = FALSE),
  CHECK (visibility IN ('public', 'private', 'priority', 'quarantined')),
  CHECK (launch_status IN ('draft', 'active', 'disabled', 'quarantined')),
  CHECK (review_status IN ('pending', 'reviewed', 'quarantined_sensitive', 'rejected'))
);

CREATE TABLE IF NOT EXISTS marketplace_agent_roles (
  id SERIAL PRIMARY KEY,
  import_batch_id INTEGER REFERENCES marketplace_import_batches(id) ON DELETE SET NULL,
  role_name VARCHAR(100) UNIQUE NOT NULL,
  display_name VARCHAR(255) NOT NULL,
  icon VARCHAR(100),
  description TEXT NOT NULL,
  prompt_tone_guidance TEXT NOT NULL,
  is_active BOOLEAN DEFAULT TRUE NOT NULL,
  public_claims_allowed BOOLEAN DEFAULT FALSE NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  CHECK (public_claims_allowed = FALSE)
);

CREATE TABLE IF NOT EXISTS marketplace_panel_templates (
  id SERIAL PRIMARY KEY,
  import_batch_id INTEGER REFERENCES marketplace_import_batches(id) ON DELETE SET NULL,
  source_system VARCHAR(100) NOT NULL,
  source_id VARCHAR(255),
  template_name VARCHAR(255) NOT NULL,
  default_role_context VARCHAR(100),
  is_premium_by_default BOOLEAN DEFAULT FALSE NOT NULL,
  premium_unlock_message TEXT,
  panel_example TEXT,
  description TEXT,
  enable_self_audit_feedback BOOLEAN DEFAULT FALSE NOT NULL,
  public_claims_allowed BOOLEAN DEFAULT FALSE NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  CHECK (public_claims_allowed = FALSE)
);

CREATE TABLE IF NOT EXISTS pace_message_templates (
  id SERIAL PRIMARY KEY,
  import_batch_id INTEGER REFERENCES marketplace_import_batches(id) ON DELETE SET NULL,
  source_system VARCHAR(100) NOT NULL,
  source_id VARCHAR(255),
  language_version VARCHAR(50) NOT NULL,
  version_tag VARCHAR(100) NOT NULL,
  is_active BOOLEAN DEFAULT FALSE NOT NULL,
  invite_email_subject TEXT NOT NULL,
  invite_email_body TEXT NOT NULL,
  invite_sms_message TEXT,
  onboarding_login_panel_headline TEXT,
  onboarding_login_panel_body TEXT,
  contact_shard_verification_success_message TEXT,
  post_shard_assignment_message TEXT,
  emergency_alert_subject TEXT,
  emergency_alert_body TEXT,
  emergency_sms_message TEXT,
  public_claims_allowed BOOLEAN DEFAULT FALSE NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  CHECK (public_claims_allowed = FALSE)
);

CREATE TABLE IF NOT EXISTS marketplace_user_installs (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  marketplace_item_id INTEGER REFERENCES marketplace_catalog_items(id) ON DELETE CASCADE,
  install_type VARCHAR(50) DEFAULT 'public_get' NOT NULL,
  source_version VARCHAR(50),
  is_priority BOOLEAN DEFAULT FALSE NOT NULL,
  priority_level VARCHAR(50) DEFAULT 'normal' NOT NULL,
  auto_update BOOLEAN DEFAULT TRUE NOT NULL,
  installed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  UNIQUE(subscriber_id, marketplace_item_id)
);

CREATE TABLE IF NOT EXISTS marketplace_ratings (
  id SERIAL PRIMARY KEY,
  marketplace_item_id INTEGER REFERENCES marketplace_catalog_items(id) ON DELETE CASCADE,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  rating INTEGER NOT NULL CHECK (rating BETWEEN 1 AND 5),
  review_text TEXT,
  verified_install BOOLEAN DEFAULT FALSE NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  UNIQUE(marketplace_item_id, subscriber_id)
);

CREATE TABLE IF NOT EXISTS marketplace_reports (
  id SERIAL PRIMARY KEY,
  marketplace_item_id INTEGER REFERENCES marketplace_catalog_items(id) ON DELETE CASCADE,
  reporter_subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE SET NULL,
  report_reason VARCHAR(100) NOT NULL,
  report_details TEXT,
  status VARCHAR(50) DEFAULT 'pending' NOT NULL,
  admin_response TEXT,
  reviewed_by INTEGER REFERENCES subscribers(id) ON DELETE SET NULL,
  reviewed_at TIMESTAMP WITH TIME ZONE,
  action_taken VARCHAR(100),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_marketplace_catalog_public
  ON marketplace_catalog_items(launch_status, review_status, visibility, contains_sensitive_info);
CREATE INDEX IF NOT EXISTS idx_marketplace_catalog_category ON marketplace_catalog_items(category);
CREATE INDEX IF NOT EXISTS idx_marketplace_catalog_object_type ON marketplace_catalog_items(object_type);
CREATE INDEX IF NOT EXISTS idx_marketplace_user_installs_subscriber ON marketplace_user_installs(subscriber_id);
CREATE INDEX IF NOT EXISTS idx_marketplace_reports_item ON marketplace_reports(marketplace_item_id);
CREATE INDEX IF NOT EXISTS idx_marketplace_import_records_batch ON marketplace_import_records(batch_id);
