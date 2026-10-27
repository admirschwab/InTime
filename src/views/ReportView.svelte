<script lang="ts">
  import { api, errorText, onDataChanged, subscribe, type Project, type Work } from '../lib/api';
  import { addDays, clockTime, dateKey, duration, longDate, parseTime, shortDate, startOfDay } from '../lib/format';
  import Calendar from '../components/Calendar.svelte';
  import Dialog from '../components/Dialog.svelte';
  import Icon from '../components/Icon.svelte';
  import Timeline from '../components/Timeline.svelte';
  import WorkDialog from '../components/WorkDialog.svelte';

  interface Entry extends Work {
    start: number;
    end: number;
    seconds: number;
  }

  interface Group {
    project: Project;
    seconds: number;
    entries: Entry[];
  }

  const today = startOfDay(new Date());
  let date = $state(today);
  let calYear = $state(today.getFullYear());
  let calMonth = $state(today.getMonth() + 1);
  let marked = $state(new Set<number>());
  let works = $state<Work[]>([]);
  let projects = $state<Project[]>([]);
  let expanded = $state(new Set<number>());
  let now = $state(Date.now());
  let editing = $state<{ work: Work | null } | null>(null);
  let deleting = $state<Work | null>(null);
  let message = $state('');
  let messageTimer: ReturnType<typeof setTimeout> | undefined;

  const isToday = $derived(dateKey(date) === dateKey(new Date(now)));

  const projectOf = (id: number): Project =>
    projects.find((p) => p.id === id) ?? {
      id,
      name: 'Unbekannt',
      description: '',
      color: '#888888',
      isWork: false,
      isDefault: false,
      isEnabled: false,
      sortIndex: 0,
    };

  const entries = $derived<Entry[]>(
    works.map((w) => {
      const start = parseTime(w.startTime);
      const end = w.active ? Math.max(start, now) : parseTime(w.endTime);
      return { ...w, start, end, seconds: (end - start) / 1000 };
    }),
  );

  const groups = $derived.by(() => {
    const map = new Map<number, Group>();
    for (const e of entries) {
      const g = map.get(e.projectId) ?? { project: projectOf(e.projectId), seconds: 0, entries: [] };
      g.seconds += e.seconds;
      g.entries.push(e);
      map.set(e.projectId, g);
    }
    return [...map.values()].sort((a, b) => a.project.sortIndex - b.project.sortIndex);
  });

  const total = $derived(entries.reduce((a, e) => a + e.seconds, 0));
  const workTotal = $derived(groups.filter((g) => g.project.isWork).reduce((a, g) => a + g.seconds, 0));
  const span = $derived(
    entries.length ? { from: entries[0].start, to: Math.max(...entries.map((e) => e.end)) } : null,
  );
  const segments = $derived(
    entries.map((e) => ({
      start: e.start,
      end: e.end,
      color: projectOf(e.projectId).color,
      label: `${projectOf(e.projectId).name}: ${clockTime(e.startTime)}–${e.active ? 'jetzt' : clockTime(e.endTime)}`,
    })),
  );

  async function loadDay() {
    try {
      [works, projects] = await Promise.all([api.worksForDay(dateKey(date)), api.projects(true)]);
    } catch (e) {
      flash(errorText(e));
    }
  }

  async function loadMarked() {
    marked = new Set(await api.daysWithWork(calYear, calMonth).catch(() => []));
  }

  $effect(() => {
    // Reload whenever the selected day / displayed month changes.
    dateKey(date);
    loadDay();
  });

  $effect(() => {
    calYear;
    calMonth;
    loadMarked();
  });

  $effect(() => {
    const tick = setInterval(() => (now = Date.now()), 15_000);
    const unlisten = subscribe(
      onDataChanged(() => {
        loadDay();
        loadMarked();
      }),
    );
    return () => {
      clearInterval(tick);
      unlisten();
    };
  });

  function selectDate(d: Date) {
    date = startOfDay(d);
    calYear = date.getFullYear();
    calMonth = date.getMonth() + 1;
    expanded = new Set();
  }

  function toggle(id: number) {
    const next = new Set(expanded);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expanded = next;
  }

  function toggleAll() {
    expanded = expanded.size === groups.length ? new Set() : new Set(groups.map((g) => g.project.id));
  }

  function flash(text: string) {
    message = text;
    clearTimeout(messageTimer);
    messageTimer = setTimeout(() => (message = ''), 3000);
  }

  /** Distinct, non-empty notes of a project, joined like in KeepTime. */
  const notesOf = (g: Group) => [...new Set(g.entries.map((e) => e.notes.trim()).filter(Boolean))].join('; ');

  async function copy(text: string, what: string) {
    try {
      await navigator.clipboard.writeText(text);
      flash(`${what} kopiert.`);
    } catch (e) {
      flash(errorText(e));
    }
  }

  function summary(): string {
    const lines = [
      `Bericht ${shortDate(date)}`,
      `Anwesend: ${duration(total)} h · Arbeit: ${duration(workTotal)} h`,
      '',
    ];
    for (const g of groups) {
      lines.push(`${g.project.name} (${duration(g.seconds)} h)`);
      const n = notesOf(g);
      if (n) lines.push(`  ${n}`);
    }
    return lines.join('\n');
  }

  async function confirmDelete() {
    const work = deleting!;
    deleting = null;
    try {
      await api.deleteWork(work.id);
    } catch (e) {
      flash(errorText(e));
    }
  }
</script>

<div class="report">
  <aside>
    <Calendar
      selected={date}
      year={calYear}
      month={calMonth}
      {marked}
      onselect={selectDate}
      onmonth={(y, m) => ((calYear = y), (calMonth = m))}
    />
    <button class="btn" disabled={isToday} onclick={() => selectDate(new Date())}>Heute</button>

    <div class="stats">
      <div class="stat">
        <span class="muted">Anwesend</span>
        <strong class="mono">{duration(total)}</strong>
      </div>
      <div class="stat">
        <span class="muted">Arbeitszeit</span>
        <strong class="mono">{duration(workTotal)}</strong>
      </div>
      <div class="stat">
        <span class="muted">Keine Arbeit</span>
        <strong class="mono">{duration(total - workTotal)}</strong>
      </div>
      {#if span}
        <div class="stat">
          <span class="muted">Zeitraum</span>
          <strong class="mono">
            {new Date(span.from).toTimeString().slice(0, 5)}–{new Date(span.to).toTimeString().slice(0, 5)}
          </strong>
        </div>
      {/if}
    </div>
  </aside>

  <main>
    <header>
      <div class="nav">
        <button class="icon-btn" title="Vorheriger Tag" onclick={() => selectDate(addDays(date, -1))}>
          <Icon name="left" />
        </button>
        <h2>{longDate(date)}</h2>
        <button class="icon-btn" title="Nächster Tag" onclick={() => selectDate(addDays(date, 1))}>
          <Icon name="right" />
        </button>
      </div>
      <div class="actions">
        <button class="btn" onclick={toggleAll} disabled={groups.length === 0}>
          {expanded.size === groups.length && groups.length ? 'Alle zuklappen' : 'Alle aufklappen'}
        </button>
        <button class="btn" onclick={() => copy(summary(), 'Zusammenfassung')} disabled={groups.length === 0}>
          <Icon name="copy" size={14} /> Kopieren
        </button>
        <button class="btn primary" onclick={() => (editing = { work: null })}>
          <Icon name="plus" size={14} /> Eintrag
        </button>
      </div>
    </header>

    <Timeline {segments} />

    {#if groups.length === 0}
      <p class="empty muted">Für diesen Tag wurde keine Zeit erfasst.</p>
    {:else}
      <div class="groups">
        {#each groups as g (g.project.id)}
          {@const open = expanded.has(g.project.id)}
          {@const n = notesOf(g)}
          <div class="group" class:open>
            <div class="group-head">
              <button class="expand" onclick={() => toggle(g.project.id)} aria-expanded={open}>
                <span class="chevron"><Icon name={open ? 'down' : 'right'} size={14} /></span>
                <span class="dot" style:background={g.project.color}></span>
                <span class="pname">
                  {g.project.name}
                  {#if !g.project.isWork}<span class="tag">keine Arbeit</span>{/if}
                </span>
                <span class="pnotes muted">{n}</span>
                <span class="pduration mono">{duration(g.seconds)}</span>
              </button>
              <button class="icon-btn" title="Notizen kopieren" disabled={!n} onclick={() => copy(n, 'Notizen')}>
                <Icon name="copy" size={14} />
              </button>
            </div>
            {#if open}
              <table>
                <tbody>
                  {#each g.entries as e (e.id)}
                    <tr>
                      <td class="mono range">
                        {clockTime(e.startTime)}–{e.active ? 'jetzt' : clockTime(e.endTime)}
                      </td>
                      <td class="mono dur">{duration(e.seconds)}</td>
                      <td class="enotes">{e.notes}</td>
                      <td class="row-actions">
                        {#if e.active}
                          <span class="tag live">läuft</span>
                        {:else}
                          <button class="icon-btn" title="Bearbeiten" onclick={() => (editing = { work: e })}>
                            <Icon name="edit" size={14} />
                          </button>
                          <button class="icon-btn" title="Löschen" onclick={() => (deleting = e)}>
                            <Icon name="trash" size={14} />
                          </button>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </main>
</div>

{#if editing}
  <WorkDialog work={editing.work} {date} {projects} onclose={() => (editing = null)} />
{/if}

{#if deleting}
  {@const w = deleting}
  <Dialog title="Eintrag löschen?" onclose={() => (deleting = null)}>
    <p class="muted">
      {projectOf(w.projectId).name}, {clockTime(w.startTime)}–{clockTime(w.endTime)} wird endgültig gelöscht.
    </p>
    {#snippet actions()}
      <button class="btn" onclick={() => (deleting = null)}>Abbrechen</button>
      <button class="btn danger" onclick={confirmDelete}>Löschen</button>
    {/snippet}
  </Dialog>
{/if}

{#if message}
  <div class="toast" role="status">{message}</div>
{/if}

<style>
  .report {
    display: flex;
    height: 100vh;
  }

  aside {
    display: flex;
    flex-direction: column;
    flex: none;
    width: 260px;
    padding: 16px;
    gap: 12px;
    overflow-y: auto;
    background: var(--surface);
    border-right: 1px solid var(--border);
  }

  .stats {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 6px;
  }

  .stat {
    display: flex;
    justify-content: space-between;
    padding: 8px 10px;
    background: var(--surface-2);
    border-radius: 6px;
  }

  .stat strong {
    font-size: 15px;
  }

  main {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    padding: 16px 20px;
    gap: 14px;
    overflow-y: auto;
  }

  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .nav {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  h2 {
    font-size: 18px;
  }

  .actions {
    display: flex;
    gap: 6px;
  }

  .empty {
    padding: 40px 0;
    text-align: center;
  }

  .groups {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .group {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .group-head {
    display: flex;
    align-items: center;
    padding-right: 8px;
  }

  .group.open .group-head {
    background: var(--surface);
  }

  .expand {
    display: flex;
    flex: 1;
    align-items: center;
    min-width: 0;
    gap: 10px;
    padding: 10px 12px;
    border: none;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .expand:hover {
    color: var(--hover-fg);
  }

  .chevron {
    display: flex;
    color: var(--muted);
  }

  .pname {
    flex: none;
    max-width: 40%;
    overflow: hidden;
    font-weight: 600;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .pnotes {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: 13px;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .pduration {
    font-weight: 600;
  }

  .tag {
    margin-left: 6px;
    padding: 1px 6px;
    border-radius: 10px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: 11px;
    font-weight: 400;
  }

  .tag.live {
    margin: 0;
    background: color-mix(in srgb, var(--accent) 25%, transparent);
    color: var(--fg);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  td {
    padding: 6px 12px;
    border-top: 1px solid var(--border);
    vertical-align: top;
  }

  .range {
    width: 110px;
    padding-left: 48px;
    white-space: nowrap;
  }

  .dur {
    width: 60px;
    color: var(--muted);
  }

  .enotes {
    white-space: pre-wrap;
    user-select: text;
  }

  .row-actions {
    width: 80px;
    padding-top: 2px;
    padding-bottom: 2px;
    text-align: right;
    white-space: nowrap;
  }

  .toast {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 60;
    padding: 8px 14px;
    border-radius: 6px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: 13px;
  }
</style>
