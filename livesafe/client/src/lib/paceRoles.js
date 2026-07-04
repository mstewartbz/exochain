export const PACE_ROLES = [
  {
    key: 'primary',
    letter: 'P',
    name: 'Primary',
    description: 'First person LiveSafe should alert according to your settings.',
    color: 'sky',
  },
  {
    key: 'alternate',
    letter: 'A',
    name: 'Alternate',
    description: 'Backup person if the Primary contact is unavailable.',
    color: 'emerald',
  },
  {
    key: 'contingent',
    letter: 'C',
    name: 'Contingent',
    description: 'Trusted fallback if the first two routes fail or more help is needed.',
    color: 'amber',
  },
  {
    key: 'emergency',
    letter: 'E',
    name: 'Emergency',
    description: 'Final urgent route when an emergency-card scan needs human follow-up.',
    color: 'rose',
  },
];

export const PACE_ROLE_ORDER = PACE_ROLES.map((role) => role.key);

export function normalizePaceRole(role) {
  const value = String(role || '').trim().toLowerCase();
  return value === 'custodial' ? 'contingent' : value;
}

export function getPaceRole(role) {
  const normalized = normalizePaceRole(role);
  return PACE_ROLES.find((item) => item.key === normalized) || PACE_ROLES[0];
}

export function sortPaceItems(items) {
  return [...items].sort((a, b) => {
    const aIndex = PACE_ROLE_ORDER.indexOf(normalizePaceRole(a.role || a.key));
    const bIndex = PACE_ROLE_ORDER.indexOf(normalizePaceRole(b.role || b.key));
    return (aIndex === -1 ? 99 : aIndex) - (bIndex === -1 ? 99 : bIndex);
  });
}
