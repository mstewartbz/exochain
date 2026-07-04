"use strict";

const PUBLIC_CONTENT_DENYLIST = new Set([
  "created_by",
  "created_by_id",
  "source_created_by",
  "source_created_by_id",
  "created_by_email",
  "creator_id",
  "creator_name",
]);

function filterPublicCatalogRows(rows = []) {
  return rows.filter(
    (row) =>
      row.visibility === "public" &&
      row.launch_status === "active" &&
      row.review_status === "reviewed" &&
      row.contains_sensitive_info !== true &&
      row.public_claims_allowed === false,
  );
}

function buildMarketplaceCatalogItemResponse(row) {
  return {
    id: row.id,
    slug: row.slug,
    object_type: row.object_type,
    category: row.category,
    title: row.title,
    summary: row.summary,
    icon: row.icon,
    tags: Array.isArray(row.tags) ? row.tags : [],
    content: sanitizeCatalogContent(row.content_json || {}),
    plan_gate: row.plan_gate,
    consent_requirement: row.consent_requirement,
    audit_behavior: row.audit_behavior,
    disablement_behavior: row.disablement_behavior,
    public_claims_allowed: false,
    historical: {
      install_count: parseInteger(row.historical_install_count),
      rating_average: parseNumber(row.historical_rating_average),
      rating_count: parseInteger(row.historical_rating_count),
    },
  };
}

function buildMarketplaceCatalogListResponse(rows = []) {
  const items = filterPublicCatalogRows(rows).map(buildMarketplaceCatalogItemResponse);
  return {
    items,
    total: items.length,
    public_claims_allowed: false,
  };
}

function buildMarketplaceRoleResponse(row) {
  return {
    role_name: row.role_name,
    display_name: row.display_name,
    icon: row.icon,
    description: row.description,
    prompt_tone_guidance: row.prompt_tone_guidance,
    public_claims_allowed: false,
  };
}

function buildMarketplaceRoleListResponse(rows = []) {
  const roles = rows
    .filter((row) => row.is_active !== false && row.public_claims_allowed === false)
    .map(buildMarketplaceRoleResponse);
  return {
    roles,
    total: roles.length,
    public_claims_allowed: false,
  };
}

function buildMarketplaceInstallResponse(row) {
  return {
    id: row.id,
    marketplace_item_id: row.marketplace_item_id,
    slug: row.slug,
    title: row.title,
    installed_at: row.installed_at,
  };
}

function buildMarketplaceLibraryResponse(rows = []) {
  return {
    installs: rows.map(buildMarketplaceInstallResponse),
    total: rows.length,
    public_claims_allowed: false,
  };
}

function buildMarketplaceReportResponse(row) {
  return {
    id: row.id,
    marketplace_item_id: row.marketplace_item_id,
    status: row.status || "pending",
    message: "Marketplace report received for review.",
  };
}

function sanitizeCatalogContent(value) {
  if (Array.isArray(value)) {
    return value.map(sanitizeCatalogContent);
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .filter(([key]) => !PUBLIC_CONTENT_DENYLIST.has(key))
        .filter(([key]) => !key.startsWith("source_"))
        .filter(([key]) => !key.startsWith("created_by"))
        .map(([key, entryValue]) => [key, sanitizeCatalogContent(entryValue)]),
    );
  }
  return value;
}

function parseInteger(value) {
  const parsed = Number.parseInt(String(value ?? "0"), 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

function parseNumber(value) {
  const parsed = Number.parseFloat(String(value ?? "0"));
  return Number.isFinite(parsed) ? parsed : 0;
}

module.exports = {
  buildMarketplaceCatalogItemResponse,
  buildMarketplaceCatalogListResponse,
  buildMarketplaceInstallResponse,
  buildMarketplaceLibraryResponse,
  buildMarketplaceReportResponse,
  buildMarketplaceRoleListResponse,
  buildMarketplaceRoleResponse,
  filterPublicCatalogRows,
  sanitizeCatalogContent,
};
