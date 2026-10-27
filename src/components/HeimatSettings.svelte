<script lang="ts">
  import { api, errorText, type HeimatConfig } from '../lib/api';
  import { tokenExpiry } from '../lib/heimat';

  let config = $state<HeimatConfig>({ url: '', token: '' });
  let saved = $state<HeimatConfig>({ url: '', token: '' });
  let busy = $state(false);
  let status = $state<{ ok: boolean; text: string } | null>(null);

  api.heimatConfig().then((c) => {
    config = { ...c };
    saved = { ...c };
  });

  const expiry = $derived(config.token.trim() ? tokenExpiry(config.token) : null);
  const expired = $derived(expiry !== null && expiry.getTime() < Date.now());
  const complete = $derived(config.url.trim() !== '' && config.token.trim() !== '');
  const dirty = $derived(config.url !== saved.url || config.token !== saved.token);
  const isConfigured = $derived(saved.url.trim() !== '' && saved.token.trim() !== '');

  async function test() {
    busy = true;
    status = null;
    try {
      const count = await api.testHeimat(config);
      status = { ok: true, text: `Verbindung erfolgreich – ${count} buchbare Aufgaben gefunden.` };
    } catch (e) {
      status = { ok: false, text: errorText(e) };
    } finally {
      busy = false;
    }
  }

  async function save() {
    status = null;
    try {
      await api.saveHeimatConfig(config);
      saved = await api.heimatConfig();
      config = { ...saved };
      status = { ok: true, text: 'Gespeichert.' };
    } catch (e) {
      status = { ok: false, text: errorText(e) };
    }
  }

  async function disconnect() {
    config = { url: '', token: '' };
    await save();
    status = { ok: true, text: 'Heimat-Verbindung entfernt.' };
  }
</script>

<p class="muted">
  Verbinde InTime mit <strong>Heimat</strong>, um deine erfassten Zeiten mit einem Klick aus dem Bericht nach Heimat
  zu übertragen. InTime schreibt nur dann in Heimat, wenn du im Bericht auf <em>Übertragen</em> klickst.
</p>

<label class="field">
  Heimat-URL
  <input type="text" placeholder="https://heimat.deine-firma.de" bind:value={config.url} spellcheck="false" />
  <span class="hint">Die Adresse, unter der du Heimat im Browser öffnest.</span>
</label>

<label class="field">
  Zugangstoken
  <textarea rows="3" class="token" bind:value={config.token} spellcheck="false" placeholder="eyJhbGciOi…"></textarea>
  <span class="hint">
    Erstelle in Heimat unter deinem Konto bzw. den API-Einstellungen ein persönliches Zugangstoken und füge es hier ein.
  </span>
  {#if config.token.trim()}
    {#if expiry}
      <span class:bad={expired} class:good={!expired}>
        {expired ? 'Abgelaufen am' : 'Gültig bis'}
        {expiry.toLocaleString('de-DE', { dateStyle: 'medium', timeStyle: 'short' })}
      </span>
    {:else}
      <span class="bad">Das sieht nicht wie ein gültiges Zugangstoken aus.</span>
    {/if}
  {/if}
</label>

<div class="buttons">
  <button class="btn" disabled={!complete || busy} onclick={test}>{busy ? 'Prüfe…' : 'Verbindung testen'}</button>
  <button class="btn primary" disabled={!dirty} onclick={save}>Speichern</button>
  {#if isConfigured && !dirty}
    <button class="btn" onclick={disconnect}>Verbindung entfernen</button>
  {/if}
</div>

{#if status}
  <p class:ok={status.ok} class:error={!status.ok}>{status.text}</p>
{/if}

<div class="mapping">
  <div>
    <strong>Projekte zuordnen</strong>
    <p class="muted">
      Lege fest, auf welche Heimat-Aufgabe jedes InTime-Projekt gebucht wird – oder lege Heimat-Aufgaben als neue
      Projekte an.
    </p>
  </div>
  <button class="btn" disabled={!isConfigured || dirty} onclick={() => api.openWindow('heimat')}>Zuordnung öffnen</button>
</div>

<style>
  .hint {
    font-size: 12px;
    color: var(--muted);
  }

  .token {
    font-family: Consolas, 'Cascadia Mono', monospace;
    font-size: 12px;
    word-break: break-all;
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .good {
    color: #3fb950;
    font-size: 12px;
  }

  .bad {
    color: var(--danger);
    font-size: 12px;
  }

  .ok {
    color: #3fb950;
  }

  p {
    margin: 0;
    font-size: 13px;
  }

  .mapping {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-top: 8px;
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .mapping div {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 4px;
  }
</style>
