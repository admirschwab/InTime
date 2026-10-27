<script lang="ts">
  import { api, errorText, type Project, type SyncRow } from '../lib/api';
  import { dateKey, duration, longDate } from '../lib/format';
  import { openHeimatDay, roundToQuarter } from '../lib/heimat';
  import Dialog from './Dialog.svelte';
  import Icon from './Icon.svelte';

  let {
    date,
    projects,
    heimatUrl,
    onclose,
  }: { date: Date; projects: Project[]; heimatUrl: string; onclose: () => void } = $props();

  interface Line extends SyncRow {
    selected: boolean;
    minutes: number;
    note: string;
    error?: string;
  }

  let lines = $state<Line[]>([]);
  let loading = $state(true);
  let syncing = $state(false);
  let loadError = $state('');
  let done = $state<'ok' | 'partial' | null>(null);

  const projectOf = (id: number) => projects.find((p) => p.id === id);
  const ready = $derived(lines.filter((l) => l.status === 'ready'));
  const chosen = $derived(ready.filter((l) => l.selected));
  const invalid = (l: Line) => !l.isNoteOptional && !l.note.trim();
  const canSync = $derived(chosen.length > 0 && !chosen.some(invalid) && !syncing);
  const chosenMinutes = $derived(chosen.reduce((a, l) => a + l.minutes, 0));
  const intimeTotal = $derived(lines.reduce((a, l) => a + l.intimeSeconds, 0));

  /** `afterSync`: show what was just booked instead of suggesting InTime's values again. */
  async function load(afterSync = false) {
    loading = true;
    loadError = '';
    try {
      const rows = await api.heimatDay(dateKey(date));
      lines = rows.map((r) => {
        // Prefill with InTime's values when they should be transferred, otherwise keep what Heimat has.
        const useHeimat = r.heimatSeconds > 0 && (afterSync || !r.suggestSync);
        return {
          ...r,
          selected: r.suggestSync && !afterSync,
          minutes: useHeimat ? Math.round(r.heimatSeconds / 60) : roundToQuarter(r.intimeSeconds),
          note: useHeimat ? r.heimatNotes : r.intimeNotes,
        };
      });
    } catch (e) {
      loadError = errorText(e);
    } finally {
      loading = false;
    }
  }

  load();

  function step(line: Line, delta: number) {
    line.minutes = Math.max(15, line.minutes + delta);
    line.selected = true;
  }

  async function sync() {
    syncing = true;
    done = null;
    try {
      const results = await api.heimatSync(
        dateKey(date),
        chosen.map((l) => ({
          taskId: l.taskId!,
          minutes: l.minutes,
          note: l.note,
          existingTimeIds: l.existingTimeIds,
        })),
      );
      const failed = new Map(results.filter((r) => r.error).map((r) => [r.taskId, r.error!]));
      if (failed.size === 0) {
        done = 'ok';
        await load(true);
      } else {
        done = 'partial';
        for (const l of lines) l.error = l.taskId !== null ? failed.get(l.taskId) : undefined;
      }
    } catch (e) {
      loadError = errorText(e);
    } finally {
      syncing = false;
    }
  }
</script>

<Dialog title="Zu Heimat übertragen – {longDate(date)}" width={940} {onclose}>
  {#if loading}
    <p class="muted center">Lade Daten aus Heimat…</p>
  {:else if loadError}
    <div class="center">
      <p class="error">{loadError}</p>
      <button class="btn" onclick={() => load()}>Erneut versuchen</button>
    </div>
  {:else if lines.length === 0}
    <p class="muted center">An diesem Tag gibt es keine Arbeitszeit und nichts in Heimat.</p>
  {:else}
    {#if done === 'ok'}
      <p class="banner ok">
        ✓ Übertragen. Bitte prüfe in Heimat, ob alles wie erwartet gebucht wurde.
      </p>
    {:else if done === 'partial'}
      <p class="banner error">Nicht alles konnte übertragen werden – siehe die markierten Einträge.</p>
    {/if}

    <div class="lines">
      {#each lines as line, i (line.taskId ?? `p${line.projectIds[0]}-${i}`)}
        <div class="line" class:off={line.status !== 'ready'} class:failed={!!line.error}>
          <div class="head">
            {#if line.status === 'ready'}
              <input type="checkbox" bind:checked={line.selected} title="Übertragen" />
            {:else}
              <span class="placeholder"></span>
            {/if}
            <div class="task">
              {#if line.taskId === null}
                <strong>Keine Heimat-Aufgabe</strong>
              {:else}
                <strong>{line.taskName}</strong>
                <span class="muted">{line.taskHolderName}</span>
              {/if}
            </div>
            {#if line.status === 'unmapped'}
              <span class="badge warn">nicht zugeordnet</span>
            {:else if line.status === 'unavailable'}
              <span class="badge warn">Aufgabe nicht verfügbar</span>
            {:else if line.status === 'heimatOnly'}
              <span class="badge">nur in Heimat</span>
            {:else if line.heimatSeconds > 0 && !line.suggestSync}
              <span class="badge good">bereits gebucht</span>
            {/if}
          </div>

          <div class="compare">
            <div>
              <span class="label">InTime</span>
              <span class="mono">{duration(line.intimeSeconds, true)}</span>
              <span class="projects">
                {#each line.projectIds as id (id)}
                  {@const p = projectOf(id)}
                  <span class="chip"><span class="dot" style:background={p?.color}></span>{p?.name ?? '?'}</span>
                {/each}
              </span>
              <span class="notes muted">{line.intimeNotes}</span>
            </div>
            <div>
              <span class="label">Heimat</span>
              <span class="mono">{line.heimatSeconds ? duration(line.heimatSeconds) : '–'}</span>
              <span class="notes muted">{line.heimatNotes}</span>
            </div>
          </div>

          {#if line.bookingHint}
            <p class="hint">💡 {line.bookingHint}</p>
          {/if}

          {#if line.status === 'ready'}
            <div class="book">
              <div class="stepper">
                <button class="icon-btn" title="15 Minuten weniger" disabled={line.minutes <= 15} onclick={() => step(line, -15)}>
                  <Icon name="minimize" size={14} />
                </button>
                <span class="mono">{duration(line.minutes * 60)}</span>
                <button class="icon-btn" title="15 Minuten mehr" onclick={() => step(line, 15)}>
                  <Icon name="plus" size={14} />
                </button>
              </div>
              <textarea
                rows="2"
                class:invalid={line.selected && invalid(line)}
                placeholder={line.isNoteOptional ? 'Notiz (optional)' : 'Notiz (in Heimat Pflicht)'}
                bind:value={line.note}
                oninput={() => (line.selected = true)}
              ></textarea>
            </div>
            {#if line.existingTimeIds.length > 0 && line.selected}
              <p class="hint">Die {line.existingTimeIds.length} vorhandene(n) Buchung(en) auf dieser Aufgabe werden ersetzt.</p>
            {/if}
          {:else if line.status === 'unmapped'}
            <p class="hint">
              Ordne das Projekt einer Heimat-Aufgabe zu, um die Zeit zu übertragen.
              <button class="link" onclick={() => api.openWindow('heimat')}>Zuordnung öffnen</button>
            </p>
          {:else if line.status === 'unavailable'}
            <p class="hint">Heimat bietet die zugeordnete Aufgabe an diesem Tag nicht (mehr) an. Prüfe die Zuordnung.</p>
          {/if}

          {#if line.error}
            <p class="error">{line.error}</p>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  {#snippet actions()}
    <button class="btn link-btn" onclick={() => openHeimatDay(heimatUrl, date)}>In Heimat öffnen ↗</button>
    <span class="summary muted">
      InTime-Arbeitszeit {duration(intimeTotal)} · zu übertragen <strong>{duration(chosenMinutes * 60)}</strong>
    </span>
    <button class="btn" onclick={onclose}>Schließen</button>
    <button class="btn primary" disabled={!canSync} onclick={sync}>
      {syncing ? 'Übertrage…' : `Übertragen (${chosen.length})`}
    </button>
  {/snippet}
</Dialog>

<style>
  .center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 30px 0;
    text-align: center;
  }

  p {
    margin: 0;
  }

  .banner {
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--surface-2);
    font-size: 13px;
  }

  .banner.ok {
    color: #3fb950;
  }

  .lines {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .line {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .line.off {
    background: var(--surface);
  }

  .line.failed {
    border-color: var(--danger);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .placeholder {
    width: 16px;
  }

  .task {
    display: flex;
    flex: 1;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }

  .task .muted {
    font-size: 12px;
  }

  .badge {
    padding: 1px 8px;
    border-radius: 10px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: 11px;
    white-space: nowrap;
  }

  .badge.warn {
    background: color-mix(in srgb, #d4a72c 25%, transparent);
    color: var(--fg);
  }

  .badge.good {
    background: color-mix(in srgb, #3fb950 22%, transparent);
    color: var(--fg);
  }

  .compare {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    padding-left: 26px;
    font-size: 13px;
  }

  .compare > div {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }

  .label {
    width: 48px;
    color: var(--muted);
    font-size: 12px;
  }

  .projects {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0 6px;
    border-radius: 10px;
    background: var(--surface-2);
    font-size: 11px;
  }

  .chip .dot {
    width: 7px;
    height: 7px;
  }

  .notes {
    flex-basis: 100%;
    padding-left: 54px;
    font-size: 12px;
    white-space: pre-wrap;
  }

  .notes:empty {
    display: none;
  }

  .hint {
    padding-left: 26px;
    color: var(--muted);
    font-size: 12px;
  }

  .book {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding-left: 26px;
  }

  .stepper {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .stepper .mono {
    min-width: 44px;
    text-align: center;
    font-weight: 600;
  }

  .book textarea {
    flex: 1;
    font-size: 13px;
  }

  textarea.invalid {
    border-color: var(--danger);
  }

  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 12px;
    text-decoration: underline;
    cursor: pointer;
  }

  .link-btn {
    margin-right: auto;
  }

  .summary {
    align-self: center;
    font-size: 13px;
  }
</style>
