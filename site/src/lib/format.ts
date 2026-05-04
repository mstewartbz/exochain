export function fmtNum(n: number): string {
  return new Intl.NumberFormat('en-US').format(n);
}

export function fmtDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toISOString().slice(0, 16).replace('T', ' ') + 'Z';
}

export function fmtDateShort(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toISOString().slice(0, 10);
}

export function shorten(s: string | undefined, n = 10): string {
  if (!s) return '—';
  if (s.length <= n + 4) return s;
  return s.slice(0, n) + '…';
}
