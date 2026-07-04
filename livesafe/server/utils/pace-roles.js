const CANONICAL_PACE_ROLE_KEYS = Object.freeze([
  "primary",
  "alternate",
  "contingent",
  "emergency",
]);

const LEGACY_PACE_ROLE_ALIASES = Object.freeze({
  custodial: "contingent",
});

const PACE_ROLE_DETAILS = Object.freeze({
  primary: Object.freeze({
    key: "primary",
    letter: "P",
    name: "Primary",
    description:
      "First person you want LiveSafe to alert in an emergency, according to your settings.",
    responsibilities: Object.freeze([
      "Be ready to receive emergency alerts according to the subscriber's settings.",
      "Help coordinate next steps if the subscriber cannot speak for themself.",
      "Accept, decline, or revoke the role based on real availability.",
    ]),
    color: "sky",
    order: 0,
  }),
  alternate: Object.freeze({
    key: "alternate",
    letter: "A",
    name: "Alternate",
    description:
      "Backup person who may be contacted if the Primary person is unavailable.",
    responsibilities: Object.freeze([
      "Be ready if the Primary person cannot respond.",
      "Help the subscriber's people stay coordinated during an emergency.",
      "Accept, decline, or revoke the role based on real availability.",
    ]),
    color: "emerald",
    order: 1,
  }),
  contingent: Object.freeze({
    key: "contingent",
    letter: "C",
    name: "Contingent",
    description:
      "Trusted person who may help if the first two routes fail or the situation needs another ready human.",
    responsibilities: Object.freeze([
      "Be a trusted fallback if Primary and Alternate routes are not enough.",
      "Help keep the Safety Circle resilient without taking over the subscriber's choices.",
      "Accept, decline, or revoke the role based on real availability.",
    ]),
    color: "amber",
    order: 2,
  }),
  emergency: Object.freeze({
    key: "emergency",
    letter: "E",
    name: "Emergency",
    description:
      "Final emergency route who may be notified when the subscriber's LiveSafe card is scanned.",
    responsibilities: Object.freeze([
      "Be ready for urgent emergency-card scan notifications.",
      "Help first responders or trusted people reach the right next contact path.",
      "Accept, decline, or revoke the role based on real availability.",
    ]),
    color: "rose",
    order: 3,
  }),
});

function normalizePaceRole(role) {
  const normalized = String(role || "").trim().toLowerCase();
  const canonical = LEGACY_PACE_ROLE_ALIASES[normalized] || normalized;

  if (!CANONICAL_PACE_ROLE_KEYS.includes(canonical)) {
    throw new Error(`Unsupported P.A.C.E. role: ${role || "(empty)"}`);
  }

  return canonical;
}

function normalizePaceRoles(roles) {
  return roles.map((role) => normalizePaceRole(role));
}

function getPaceRoleDetails(role) {
  return PACE_ROLE_DETAILS[normalizePaceRole(role)];
}

function comparePaceRoles(a, b) {
  return getPaceRoleDetails(a).order - getPaceRoleDetails(b).order;
}

module.exports = {
  CANONICAL_PACE_ROLE_KEYS,
  LEGACY_PACE_ROLE_ALIASES,
  PACE_ROLE_DETAILS,
  comparePaceRoles,
  getPaceRoleDetails,
  normalizePaceRole,
  normalizePaceRoles,
};
