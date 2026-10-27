import { listen } from '@tauri-apps/api/event';
import { api, type Settings } from './api';

/** Settings shared by all components of a window; kept in sync across windows. */
export const settings = $state<{ value: Settings | null }>({ value: null });

function applyTheme(s: Settings) {
  const root = document.documentElement.style;
  root.setProperty('--bg', s.backgroundColor);
  root.setProperty('--fg', s.textColor);
  root.setProperty('--hover-bg', s.hoverBackgroundColor);
  root.setProperty('--hover-fg', s.hoverTextColor);
}

export function setSettings(s: Settings) {
  settings.value = s;
  applyTheme(s);
}

export async function initSettings() {
  setSettings(await api.settings());
  await listen<Settings>('settings-changed', (e) => setSettings(e.payload));
}
