"use strict";

const crypto = require("crypto");
const fs = require("fs");
const path = require("path");

const IMPORT_SOURCE_LABEL = "ambientli-export-2026-06-22";
const SAFE_PUBLIC_KEY_DENYLIST = new Set([
  "created_by",
  "created_by_id",
  "source_created_by",
  "source_created_by_id",
  "created_by_email",
  "created_by_user_id",
  "creator_email",
  "creator_user_id",
]);

function computeFileSha256(filePath) {
  return crypto.createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
}

function normalizeSlug(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .replace(/-{2,}/g, "-");
}

function buildAmbientliImportPlan({ zipPath, exportPath, sampleDataReviewed = false }) {
  const exportJson = JSON.parse(fs.readFileSync(exportPath, "utf8"));
  const entities = exportJson.entities || {};
  const entityCounts = buildEntityCounts(entities);
  const fieldInventory = buildFieldInventory(entities);
  const source = {
    label: IMPORT_SOURCE_LABEL,
    zip_path: zipPath,
    export_path: exportPath,
    zip_sha256: computeFileSha256(zipPath),
    export_sha256: computeFileSha256(exportPath),
    exported_at: exportJson.exported_at || null,
  };

  const { catalogItems, quarantinedRecords } = normalizeCatalogItems(
    entities.ObjectMarketplace || [],
    source,
    { sampleDataReviewed },
  );

  return {
    source,
    sampleDataReviewed,
    entityCounts,
    fieldInventory,
    catalogItems,
    quarantinedRecords,
    agentRoles: normalizeAgentRoles(entities.AIRoleDefinition || []),
    panelTemplates: normalizePanelTemplates(entities.PanelTemplateSetting || []),
    paceMessageTemplates: normalizePaceMessageTemplates(entities.PaceMessagingConfig || []),
    importRecords: buildImportRecords(entities, source, { sampleDataReviewed }),
  };
}

function buildEntityCounts(entities) {
  return Object.fromEntries(
    Object.entries(entities).map(([entity, rows]) => [
      entity,
      Array.isArray(rows) ? rows.length : 0,
    ]),
  );
}

function buildFieldInventory(entities) {
  return Object.fromEntries(
    Object.entries(entities).map(([entity, rows]) => {
      const fields = new Set();
      if (Array.isArray(rows)) {
        for (const row of rows) {
          if (row && typeof row === "object" && !Array.isArray(row)) {
            Object.keys(row).forEach((key) => fields.add(key));
          }
        }
      }
      return [entity, Array.from(fields).sort()];
    }),
  );
}

function normalizeCatalogItems(rows, source, { sampleDataReviewed = false } = {}) {
  const catalogItems = [];
  const quarantinedRecords = [];
  const slugsSeen = new Map();

  for (const row of rows) {
    const reasons = quarantineReasons(row, { sampleDataReviewed });
    if (reasons.length > 0) {
      quarantinedRecords.push({
        entity: "ObjectMarketplace",
        source_id: row.id || null,
        title: row.title || "(untitled)",
        reviewStatus: "quarantined_sensitive",
        reasons,
        safeExcerpt: buildSafeExcerpt(row),
      });
      continue;
    }

    const baseSlug = normalizeSlug(row.title || row.id || "marketplace-item");
    const slug = uniqueSlug(baseSlug, slugsSeen);
    const content = sanitizePublicValue(row.object_data || {});
    const tags = Array.isArray(row.tags)
      ? row.tags.map((tag) => normalizePublicString(tag)).filter(Boolean)
      : [];

    catalogItems.push({
      source_system: "imported-marketplace",
      source_id: row.id,
      source_sha256: source.export_sha256,
      slug,
      object_type: normalizePublicString(row.object_type || "template"),
      category: normalizePublicString(row.category || "other"),
      title: normalizePublicString(row.title || "Untitled template"),
      summary: normalizePublicString(row.ai_generated_description || row.object_data?.objective || ""),
      icon: normalizePublicString(row.ai_generated_icon || iconFor(row)),
      tags,
      content_json: content,
      plan_gate: planGateFor(row),
      consent_requirement: consentRequirementFor(row),
      audit_behavior: auditBehaviorFor(row),
      disablement_behavior: disablementBehaviorFor(row),
      visibility: "public",
      launch_status: "active",
      review_status: "reviewed",
      contains_sensitive_info: false,
      public_claims_allowed: false,
      historical_install_count: numberOrZero(row.install_count),
      historical_rating_average: numberOrZero(row.rating_average),
      historical_rating_count: numberOrZero(row.rating_count),
      historical_report_count: numberOrZero(row.report_count),
      imported_from: "source-export-2026-06-22",
    });
  }

  return { catalogItems, quarantinedRecords };
}

function quarantineReasons(row, { sampleDataReviewed = false } = {}) {
  const reasons = [];
  const reviewedPrioritySample = sampleDataReviewed && row.visibility === "priority";
  if (row.visibility !== "public" && !reviewedPrioritySample) {
    reasons.push(`visibility is ${row.visibility || "unset"}`);
  }
  if (row.contains_sensitive_info === true && !sampleDataReviewed) {
    reasons.push("contains_sensitive_info is true");
  }
  if (row.is_disabled === true) {
    reasons.push("is_disabled is true");
  }
  return reasons;
}

function uniqueSlug(baseSlug, slugsSeen) {
  const safeBase = baseSlug || "marketplace-item";
  const count = slugsSeen.get(safeBase) || 0;
  slugsSeen.set(safeBase, count + 1);
  return count === 0 ? safeBase : `${safeBase}-${count + 1}`;
}

function normalizeAgentRoles(rows) {
  const byRole = new Map();
  for (const row of rows) {
    if (!row.isActive || !row.roleName) {
      continue;
    }
    if (!byRole.has(row.roleName)) {
      byRole.set(row.roleName, {
        role_name: normalizePublicString(row.roleName),
        display_name: normalizePublicString(row.displayName || row.roleName),
        icon: normalizePublicString(row.icon || "Sparkles"),
        description: normalizePublicString(row.description || ""),
        prompt_tone_guidance: normalizePublicString(row.promptToneGuidance || ""),
        is_active: true,
        public_claims_allowed: false,
      });
    }
  }
  return Array.from(byRole.values()).sort((left, right) =>
    left.role_name.localeCompare(right.role_name),
  );
}

function normalizePanelTemplates(rows) {
  return rows.map((row) => ({
    source_system: "imported-marketplace",
    source_id: row.id || null,
    template_name: normalizePublicString(row.templateName || "Untitled panel"),
    default_role_context: normalizePublicString(row.defaultRoleContext || ""),
    is_premium_by_default: row.isPremiumByDefault === true,
    premium_unlock_message: normalizePublicString(
      row.premiumUnlockMessage || "Upgrade to unlock this guidance.",
    ),
    panel_example: normalizePublicString(row.panelExample || ""),
    description: normalizePublicString(row.description || ""),
    enable_self_audit_feedback: row.enableSelfAuditFeedback === true,
    public_claims_allowed: false,
  }));
}

function normalizePaceMessageTemplates(rows) {
  return rows
    .filter((row) => row && row.isActive !== false)
    .map((row) => ({
      source_system: "imported-marketplace",
      source_id: row.id || null,
      language_version: normalizePublicString(row.languageVersion || "en-US"),
      version_tag: normalizePublicString(
        rewriteBrand(row.versionTag || "livesafe-default-1.0"),
      ),
      is_active: true,
      invite_email_subject: normalizePublicString(
        rewriteBrand(row.inviteEmailSubject || ""),
      ),
      invite_email_body: normalizePublicString(rewriteBrand(row.inviteEmailBody || "")),
      invite_sms_message: normalizePublicString(rewriteBrand(row.inviteSMSMessage || "")),
      onboarding_login_panel_headline: normalizePublicString(
        rewriteBrand(row.onboardingLoginPanelHeadline || ""),
      ),
      onboarding_login_panel_body: normalizePublicString(
        rewriteBrand(row.onboardingLoginPanelBody || ""),
      ),
      contact_shard_verification_success_message: normalizePublicString(
        rewriteBrand(row.contactShardVerificationSuccessMessage || ""),
      ),
      post_shard_assignment_message: normalizePublicString(
        rewriteBrand(row.postShardAssignmentMessage || ""),
      ),
      emergency_alert_subject: normalizePublicString(
        rewriteBrand(row.emergencyAlertSubject || ""),
      ),
      emergency_alert_body: normalizePublicString(rewriteBrand(row.emergencyAlertBody || "")),
      emergency_sms_message: normalizePublicString(
        rewriteBrand(row.emergencySMSMessage || ""),
      ),
      public_claims_allowed: false,
    }));
}

function buildImportRecords(entities, source, { sampleDataReviewed = false } = {}) {
  const records = [];
  for (const [entity, rows] of Object.entries(entities)) {
    if (!Array.isArray(rows)) {
      continue;
    }
    for (const row of rows) {
      records.push({
        source_system: "ambientli",
        source_entity: entity,
        source_id: row?.id || null,
        source_sha256: source.export_sha256,
        review_status:
          entity === "ObjectMarketplace" &&
          quarantineReasons(row, { sampleDataReviewed }).length > 0
            ? "quarantined_sensitive"
            : "reviewed",
        safe_excerpt: buildSafeExcerpt(row),
      });
    }
  }
  return records;
}

function buildSafeExcerpt(row) {
  return sanitizePublicValue({
    id: row?.id || null,
    title: row?.title || row?.templateName || row?.roleName || null,
    object_type: row?.object_type || null,
    category: row?.category || null,
    visibility: row?.visibility || null,
    contains_sensitive_info: row?.contains_sensitive_info === true,
  });
}

function sanitizePublicValue(value) {
  if (Array.isArray(value)) {
    return value.map(sanitizePublicValue);
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .filter(([key]) => !SAFE_PUBLIC_KEY_DENYLIST.has(key))
        .filter(([key]) => !key.startsWith("created_by"))
        .map(([key, entryValue]) => [key, sanitizePublicValue(entryValue)]),
    );
  }
  if (typeof value === "string") {
    return normalizePublicString(rewriteBrand(value));
  }
  return value;
}

function normalizePublicString(value) {
  return rewriteBrand(String(value ?? "").trim());
}

function rewriteBrand(value) {
  return String(value)
    .replace(/ambient\.li/gi, "LiveSafe.ai")
    .replace(/ambientli/gi, "LiveSafe.ai");
}

function iconFor(row) {
  if (row.object_type === "emergency") {
    return "ShieldAlert";
  }
  if (row.object_type === "meeting") {
    return "Users";
  }
  return "Sparkles";
}

function planGateFor(row) {
  const tags = Array.isArray(row.tags) ? row.tags.map((tag) => String(tag).toLowerCase()) : [];
  if (tags.includes("premium") || row.category === "legal") {
    return "family_or_higher";
  }
  return "basic_or_higher";
}

function consentRequirementFor(row) {
  if (row.object_type === "emergency" || row.category === "medical") {
    return "emergency_outreach_acknowledgement";
  }
  if (row.category === "emergency" || row.category === "personal") {
    return "household_coordination_acknowledgement";
  }
  return "none";
}

function auditBehaviorFor(row) {
  if (row.object_type === "emergency") {
    return "rule_execution_audit";
  }
  if (row.category === "legal") {
    return "governance_audit_trail";
  }
  return "access_log_only";
}

function disablementBehaviorFor(row) {
  if (row.object_type === "emergency") {
    return "revoke_scheduled_actions_retain_audit";
  }
  if (row.category === "legal") {
    return "freeze_rules_retain_audit";
  }
  return "disable_future_runs_retain_audit";
}

function numberOrZero(value) {
  const numberValue = Number(value);
  return Number.isFinite(numberValue) ? numberValue : 0;
}

function defaultArtifactPaths() {
  return {
    zipPath:
      process.env.AMBIENTLI_ZIP_PATH ||
      path.join(
        process.env.HOME || "/Users/bobstewart",
        "Downloads",
        "ambientli-7a29f7df (2).zip",
      ),
    exportPath:
      process.env.AMBIENTLI_EXPORT_PATH ||
      path.join(
        process.env.HOME || "/Users/bobstewart",
        "Downloads",
        "ambient_export_2026-06-22 (1).json",
      ),
  };
}

module.exports = {
  IMPORT_SOURCE_LABEL,
  buildAmbientliImportPlan,
  computeFileSha256,
  defaultArtifactPaths,
  normalizeSlug,
  sanitizePublicValue,
};
