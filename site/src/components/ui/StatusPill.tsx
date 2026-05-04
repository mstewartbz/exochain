import { Pill } from './Pill';

export type Status =
  | 'active'
  | 'expired'
  | 'revoked'
  | 'quarantined'
  | 'inactive'
  | 'healthy'
  | 'degraded'
  | 'syncing'
  | 'offline'
  | 'permitted'
  | 'denied'
  | 'partial'
  | 'open'
  | 'mitigated'
  | 'resolved'
  | 'draft'
  | 'ratified'
  | 'rejected'
  | 'success'
  | 'error';

const map: Record<Status, Parameters<typeof Pill>[0]['tone']> = {
  active: 'verify',
  expired: 'roadmap',
  revoked: 'alert',
  quarantined: 'signal',
  inactive: 'roadmap',
  healthy: 'verify',
  degraded: 'signal',
  syncing: 'custody',
  offline: 'alert',
  permitted: 'verify',
  denied: 'alert',
  partial: 'signal',
  open: 'signal',
  mitigated: 'custody',
  resolved: 'verify',
  draft: 'roadmap',
  ratified: 'verify',
  rejected: 'alert',
  success: 'verify',
  error: 'alert'
};

export function StatusPill({ status }: { status: Status }) {
  return <Pill tone={map[status]}>{status}</Pill>;
}
