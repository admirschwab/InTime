const pad = (n: number) => String(n).padStart(2, '0');

/** `YYYY-MM-DD` of a local date. */
export function dateKey(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Parses a backend timestamp (`YYYY-MM-DDTHH:MM:SS`, local time) to epoch millis. */
export function parseTime(s: string): number {
  return new Date(s).getTime();
}

/** `HH:MM` of a backend timestamp. */
export function clockTime(s: string): string {
  return s.slice(11, 16);
}

/** `h:mm`, or `hh:mm:ss` with seconds. */
export function duration(seconds: number, withSeconds = false): string {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return withSeconds ? `${pad(h)}:${pad(m)}:${pad(s % 60)}` : `${h}:${pad(m)}`;
}

export function longDate(d: Date): string {
  return d.toLocaleDateString('de-DE', { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' });
}

export function shortDate(d: Date): string {
  return d.toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit', year: 'numeric' });
}

export function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}

export function addDays(d: Date, days: number): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + days);
}
