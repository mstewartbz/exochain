function buildPublicRecordProviderResponse(row = {}) {
  return {
    id: row.id ?? null,
    provider_name: row.provider_name ?? null,
    npi: row.npi ?? row.provider_npi ?? null,
    facility: row.facility ?? row.provider_facility ?? null,
    specialty: row.specialty ?? row.provider_specialty ?? null,
    npi_verified: Boolean(row.npi_verified),
  };
}

function buildPublicRecordProviderListResponse(rows = []) {
  return rows.map((row) => buildPublicRecordProviderResponse(row));
}

module.exports = {
  buildPublicRecordProviderResponse,
  buildPublicRecordProviderListResponse,
};
