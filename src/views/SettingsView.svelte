<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';
  import { ask, open, save } from '@tauri-apps/plugin-dialog';
  import { api, DEFAULT_COLORS, errorText, type Settings } from '../lib/api';
  import { dateKey } from '../lib/format';
  import { setSettings, settings } from '../lib/settings.svelte';
  import HeimatSettings from '../components/HeimatSettings.svelte';
  import Icon from '../components/Icon.svelte';

  const SECTIONS = [
    { id: 'colors', label: 'Farben' },
    { id: 'display', label: 'Darstellung' },
    { id: 'behavior', label: 'Verhalten' },
    { id: 'heimat', label: 'Heimat API' },
    { id: 'data', label: 'Daten' },
  ] as const;

  // svelte-ignore state_referenced_locally
  let form = $state<Settings>({ ...settings.value! });
  let section = $state<(typeof SECTIONS)[number]['id']>('colors');
  let message = $state('');
  let error = $state('');
  let version = $state('');

  getVersion().then((v) => (version = v));

  const colors: { key: keyof typeof DEFAULT_COLORS; label: string }[] = [
    { key: 'backgroundColor', label: 'Hintergrund' },
    { key: 'textColor', label: 'Text' },
    { key: 'hoverBackgroundColor', label: 'Hover-Hintergrund' },
    { key: 'hoverTextColor', label: 'Hover-Text' },
  ];

  async function persist() {
    error = '';
    try {
      setSettings({ ...form });
      await api.saveSettings({ ...form });
    } catch (e) {
      error = errorText(e);
    }
  }

  async function exportData() {
    message = error = '';
    const path = await save({
      defaultPath: `intime-export-${dateKey(new Date())}.json`,
      filters: [{ name: 'InTime-Export', extensions: ['json'] }],
    });
    if (!path) return;
    try {
      await api.exportData(path);
      message = `Exportiert nach ${path}`;
    } catch (e) {
      error = errorText(e);
    }
  }

  async function importData() {
    message = error = '';
    const path = await open({ multiple: false, filters: [{ name: 'InTime-Export', extensions: ['json'] }] });
    if (!path) return;
    const confirmed = await ask(
      'Alle aktuellen Projekte, Zeiten und Einstellungen werden durch den Inhalt der Datei ersetzt. InTime startet danach neu.',
      { title: 'Daten importieren?', kind: 'warning', okLabel: 'Importieren', cancelLabel: 'Abbrechen' },
    );
    if (!confirmed) return;
    try {
      await api.importData(path);
    } catch (e) {
      error = errorText(e);
    }
  }
</script>

<div class="settings">
  <nav>
    {#each SECTIONS as s (s.id)}
      <button class:active={section === s.id} onclick={() => ((section = s.id), (message = error = ''))}>
        {s.label}
      </button>
    {/each}
    <span class="version muted">InTime {version}</span>
  </nav>

  <main>
    {#if section === 'colors'}
      <h2>Farben</h2>
      <div class="colors">
        {#each colors as c (c.key)}
          <div class="color-row">
            <input type="color" bind:value={form[c.key]} oninput={persist} />
            <span>{c.label}</span>
            <button
              class="btn small"
              disabled={form[c.key] === DEFAULT_COLORS[c.key]}
              onclick={() => ((form[c.key] = DEFAULT_COLORS[c.key]), persist())}
            >
              Zurücksetzen
            </button>
          </div>
        {/each}
      </div>
    {:else if section === 'display'}
      <h2>Darstellung</h2>
      <label class="check">
        <input type="checkbox" bind:checked={form.displayProjectsRight} onchange={persist} />
        Projektliste rechts anzeigen
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={form.hideProjectsOnMouseExit} onchange={persist} />
        Projektliste ausblenden, wenn die Maus das Fenster verlässt
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={form.saveWindowPosition} onchange={persist} />
        Fensterposition beim Start wiederherstellen
      </label>
    {:else if section === 'behavior'}
      <h2>Verhalten</h2>
      <label class="check">
        <input type="checkbox" bind:checked={form.remindNotes} onchange={persist} />
        Beim Projektwechsel nach einer Notiz fragen, wenn keine eingetragen ist
      </label>
      <label class="check indent" class:disabled={!form.remindNotes}>
        <input
          type="checkbox"
          bind:checked={form.remindNotesOnlyWork}
          onchange={persist}
          disabled={!form.remindNotes}
        />
        Nur bei Projekten, die als Arbeitszeit zählen
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={form.confirmClose} onchange={persist} />
        Vor dem Beenden nachfragen
      </label>
    {:else if section === 'heimat'}
      <h2>Heimat API</h2>
      <HeimatSettings />
    {:else if section === 'data'}
      <h2>Daten</h2>
      <p class="muted">Sichere deine Daten als JSON-Datei. Beim Import werden alle vorhandenen Daten ersetzt.</p>
      <div class="buttons">
        <button class="btn" onclick={exportData}><Icon name="download" size={14} /> Exportieren</button>
        <button class="btn" onclick={importData}><Icon name="upload" size={14} /> Importieren</button>
      </div>
      {#if message}<p class="ok">{message}</p>{/if}
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
  </main>
</div>

<style>
  .settings {
    display: flex;
    height: 100vh;
  }

  nav {
    display: flex;
    flex-direction: column;
    flex: none;
    width: 190px;
    padding: 16px 10px;
    gap: 2px;
    background: var(--surface);
    border-right: 1px solid var(--border);
  }

  nav button {
    padding: 8px 12px;
    border: none;
    border-left: 3px solid transparent;
    border-radius: 4px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  nav button:hover {
    background: var(--hover-bg);
    color: var(--hover-fg);
  }

  nav button.active {
    border-left-color: var(--accent);
    background: var(--surface-2);
    font-weight: 600;
  }

  .version {
    margin-top: auto;
    padding: 0 12px;
    font-size: 12px;
  }

  main {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    padding: 20px 24px;
    gap: 12px;
    overflow-y: auto;
  }

  h2 {
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
    font-size: 16px;
  }

  .colors {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .color-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .color-row span {
    flex: 1;
  }

  input[type='color'] {
    width: 36px;
    height: 26px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: none;
    cursor: pointer;
  }

  .btn.small {
    padding: 3px 10px;
    font-size: 12px;
  }

  .indent {
    padding-left: 24px;
  }

  .disabled {
    opacity: 0.5;
  }

  p {
    margin: 0;
    font-size: 13px;
  }

  .buttons {
    display: flex;
    gap: 8px;
  }

  .ok {
    color: #3fb950;
    word-break: break-all;
  }
</style>
