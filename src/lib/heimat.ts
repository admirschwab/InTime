import { openUrl } from '@tauri-apps/plugin-opener';

/** Expiry date of a Heimat access token (a JWT with an `expiration` claim), if readable. */
export function tokenExpiry(token: string): Date | null {
  try {
    const payload = token.trim().replace(/^Bearer\s+/i, '').split('.')[1];
    const json = JSON.parse(atob(payload.replace(/-/g, '+').replace(/_/g, '/')));
    if (typeof json.expiration === 'string') return new Date(json.expiration.replace(' ', 'T'));
    if (typeof json.exp === 'number') return new Date(json.exp * 1000);
  } catch {
    // not a JWT
  }
  return null;
}

/** Opens the given day in the Heimat web app. */
export function openHeimatDay(url: string, date: Date) {
  const base = url.trim().replace(/\/+$/, '');
  return openUrl(`${base}/core/heimat/time/main/day/${date.getFullYear()}/${date.getMonth() + 1}/${date.getDate()}`);
}

/** Heimat only accepts multiples of 15 minutes; rounds to the nearest quarter, at least 15. */
export function roundToQuarter(seconds: number): number {
  return Math.max(15, Math.round(seconds / 60 / 15) * 15);
}
