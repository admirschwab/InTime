<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { api, errorText, type HeimatMapping, type HeimatTask, type Project } from '../lib/api';

  let projects = $state<Project[]>([]);
  let tasks = $state<HeimatTask[]>([]);
  /** Selected task per project id; `null` = not mapped. */
  let selection = $state<Record<number, number | null>>({});
  /** Stored mappings, used for tasks that Heimat no longer offers. */
  let stored = $state<HeimatMapping[]>([]);
  let toImport = $state(new Set<number>());
  let loading = $state(true);
  let error = $state('');
  let message = $state('');

  const groups = $derived.by(() => {
    const map = new Map<string, HeimatTask[]>();
    for (const t of tasks) map.set(t.taskHolderName, [...(map.get(t.taskHolderName) ?? []), t]);
    return [...map.entries()];
  });
  const mappedTaskIds = $derived(new Set(Object.values(selection).filter((id): id is number => id !== null)));
  const unmappedTasks = $derived(tasks.filter((t) => !mappedTaskIds.has(t.id)));
  const isAvailable = (taskId: number) => tasks.some((t) => t.id === taskId);
  const storedFor = (projectId: number) => stored.find((m) => m.projectId === projectId);

  async function load() {
    loading = true;
    error = '';
    try {
      const [projectList, taskList, mappings] = await Promise.all([
        api.projects(false),
        api.heimatTasks(),
        api.heimatMappings(),
      ]);
      projects = projectList;
      tasks = taskList;
      stored = mappings;
      selection = Object.fromEntries(
        projectList.map((p) => [p.id, mappings.find((m) => m.projectId === p.id)?.taskId ?? null]),
      );
      toImport = new Set();
    } catch (e) {
      error = errorText(e);
    } finally {
      loading = false;
    }
  }

  load();

  function currentMappings(): HeimatMapping[] {
    return projects.flatMap((p) => {
      const taskId = selection[p.id];
      if (taskId == null) return [];
      const task = tasks.find((t) => t.id === taskId);
      const old = storedFor(p.id);
      return [
        {
          projectId: p.id,
          taskId,
          taskName: task?.name ?? old?.taskName ?? '',
          taskHolderName: task?.taskHolderName ?? old?.taskHolderName ?? '',
        },
      ];
    });
  }

  async function save(close: boolean) {
    error = message = '';
    try {
      await api.saveHeimatMappings(currentMappings());
      if (close) await getCurrentWindow().close();
      else message = 'Zuordnung gespeichert.';
    } catch (e) {
      error = errorText(e);
    }
  }

  async function importSelected() {
    error = message = '';
    const selected = tasks.filter((t) => toImport.has(t.id));
    try {
      // Keep edits made above, then create the new projects.
      await api.saveHeimatMappings(currentMappings());
      await api.importHeimatTasks(selected);
      await load();
      message = `${selected.length} ${selected.length === 1 ? 'Projekt' : 'Projekte'} angelegt und zugeordnet.`;
    } catch (e) {
      error = errorText(e);
    }
  }

  function toggleImport(id: number) {
    const next = new Set(toImport);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    toImport = next;
  }

  function onSelect(projectId: number, value: string) {
    selection[projectId] = value === '' ? null : Number(value);
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && getCurrentWindow().close()} />

<div class="heimat">
  <header>
    <h2>Heimat-Zuordnung</h2>
    <p class="muted">
      Wähle für jedes Projekt, auf welche Heimat-Aufgabe seine Zeit gebucht wird. Mehrere Projekte dürfen dieselbe
      Aufgabe nutzen – ihre Zeiten werden dann zusammengezählt. Projekte ohne Zuordnung werden nicht übertragen.
    </p>
  </header>

  {#if loading}
    <p class="muted center">Lade Aufgaben aus Heimat…</p>
  {:else if error && tasks.length === 0}
    <div class="center">
      <p class="error">{error}</p>
      <button class="btn" onclick={load}>Erneut versuchen</button>
    </div>
  {:else}
    <section>
      <h3>InTime-Projekte</h3>
      <table>
        <tbody>
          {#each projects as p (p.id)}
            {@const taskId = selection[p.id]}
            {@const missing = taskId != null && !isAvailable(taskId)}
            <tr>
              <td class="project">
                <span class="dot" style:background={p.color}></span>
                <span>{p.name}</span>
                {#if !p.isWork}<span class="tag">keine Arbeit</span>{/if}
              </td>
              <td>
                <select
                  class:warn={missing}
                  value={taskId == null ? '' : String(taskId)}
                  onchange={(e) => onSelect(p.id, e.currentTarget.value)}
                >
                  <option value="">— nicht zugeordnet —</option>
                  {#if missing}
                    <option value={String(taskId)}>
                      ⚠ {storedFor(p.id)?.taskHolderName} – {storedFor(p.id)?.taskName} (nicht mehr verfügbar)
                    </option>
                  {/if}
                  {#each groups as [holder, list] (holder)}
                    <optgroup label={holder}>
                      {#each list as t (t.id)}
                        <option value={String(t.id)}>{t.name}</option>
                      {/each}
                    </optgroup>
                  {/each}
                </select>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="muted small">Projekte mit „keine Arbeit“ (z. B. Pause) werden nie übertragen.</p>
    </section>

    <section>
      <h3>Heimat-Aufgaben als neue Projekte anlegen</h3>
      {#if unmappedTasks.length === 0}
        <p class="muted small">Alle Heimat-Aufgaben sind bereits einem Projekt zugeordnet.</p>
      {:else}
        <div class="import-list">
          {#each unmappedTasks as t (t.id)}
            <label class="check" title={t.bookingHint ?? ''}>
              <input type="checkbox" checked={toImport.has(t.id)} onchange={() => toggleImport(t.id)} />
              <span><span class="muted">{t.taskHolderName} –</span> {t.name}</span>
            </label>
          {/each}
        </div>
        <div>
          <button class="btn" disabled={toImport.size === 0} onclick={importSelected}>
            {toImport.size} als Projekt anlegen
          </button>
        </div>
      {/if}
    </section>
  {/if}

  <footer>
    {#if message}<span class="ok">{message}</span>{/if}
    {#if error && tasks.length > 0}<span class="error">{error}</span>{/if}
    <span class="spacer"></span>
    <button class="btn" onclick={() => getCurrentWindow().close()}>Schließen</button>
    <button class="btn primary" disabled={loading || tasks.length === 0} onclick={() => save(true)}>Speichern</button>
  </footer>
</div>

<style>
  .heimat {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 18px 22px 0;
    gap: 16px;
    overflow-y: auto;
  }

  header {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  h2 {
    font-size: 18px;
  }

  h3 {
    margin-bottom: 8px;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }

  p {
    margin: 0;
    font-size: 13px;
  }

  .small {
    margin-top: 6px;
    font-size: 12px;
  }

  .center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 40px 0;
    text-align: center;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  td {
    padding: 5px 0;
    border-bottom: 1px solid var(--border);
  }

  .project {
    width: 40%;
    padding-right: 12px;
  }

  .project > span:nth-child(2) {
    margin-left: 8px;
  }

  .tag {
    margin-left: 6px;
    padding: 1px 6px;
    border-radius: 10px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: 11px;
  }

  select.warn {
    border-color: #d4a72c;
  }

  .import-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 6px 16px;
    margin-bottom: 10px;
    font-size: 13px;
  }

  footer {
    position: sticky;
    bottom: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: auto;
    padding: 12px 0 16px;
    background: var(--bg);
    border-top: 1px solid var(--border);
    font-size: 13px;
  }

  .spacer {
    flex: 1;
  }

  .ok {
    color: #3fb950;
  }
</style>
