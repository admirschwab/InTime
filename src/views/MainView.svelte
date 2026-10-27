<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow, LogicalSize, PhysicalPosition } from '@tauri-apps/api/window';
  import { api, errorText, onDataChanged, subscribe, type Current, type Project } from '../lib/api';
  import { duration, parseTime } from '../lib/format';
  import { settings } from '../lib/settings.svelte';
  import Dialog from '../components/Dialog.svelte';
  import Icon from '../components/Icon.svelte';

  const MAIN_WIDTH = 300;
  const LIST_WIDTH = 240;
  const HEIGHT = 320;

  type DialogState =
    | { kind: 'notes'; project: Project; transfer: number }
    | { kind: 'transfer'; project: Project }
    | { kind: 'delete'; project: Project }
    | { kind: 'close' };

  const win = getCurrentWindow();
  const s = $derived(settings.value!);

  let allProjects = $state<Project[]>([]);
  let current = $state<Current | null>(null);
  let notes = $state('');
  let now = $state(Date.now());
  let search = $state('');
  let collapsed = $state(false);
  let menu = $state<{ project: Project; x: number; y: number } | null>(null);
  let dialog = $state<DialogState | null>(null);
  let reminderNotes = $state('');
  let transferMinutes = $state(5);
  let error = $state('');

  let notesTimer: ReturnType<typeof setTimeout> | undefined;
  let errorTimer: ReturnType<typeof setTimeout> | undefined;
  let collapseTimer: ReturnType<typeof setTimeout> | undefined;
  let searchInput: HTMLInputElement | undefined = $state();

  const projects = $derived(allProjects.filter((p) => p.isEnabled));
  const filtered = $derived(
    search.trim() ? projects.filter((p) => p.name.toLowerCase().includes(search.trim().toLowerCase())) : projects,
  );
  const active = $derived(allProjects.find((p) => p.id === current?.work.projectId));
  const elapsed = $derived(current ? (now - parseTime(current.work.startTime)) / 1000 : 0);

  /** Today's seconds per project, including the running work. */
  const todayByProject = $derived.by(() => {
    const map = new Map<number, number>();
    for (const t of current?.today ?? []) map.set(t.projectId, t.seconds);
    if (current) map.set(current.work.projectId, (map.get(current.work.projectId) ?? 0) + elapsed);
    return map;
  });
  const todayTotal = $derived([...todayByProject.values()].reduce((a, b) => a + b, 0));
  const todayWork = $derived(
    [...todayByProject.entries()]
      .filter(([id]) => allProjects.find((p) => p.id === id)?.isWork)
      .reduce((a, [, sec]) => a + sec, 0),
  );
  const maxTransfer = $derived(Math.floor(elapsed / 60));

  // Window title doubles as the taskbar label.
  const title = $derived(active ? `${duration(elapsed)} · ${active.name} – InTime` : 'InTime');
  $effect(() => {
    win.setTitle(title);
  });

  async function load() {
    try {
      const [projectList, cur] = await Promise.all([api.projects(true), api.current()]);
      // Keep locally typed notes unless a different work is running now.
      if (cur.work.id !== current?.work.id) notes = cur.work.notes;
      allProjects = projectList;
      current = cur;
    } catch (e) {
      showError(e);
    }
  }

  $effect(() => {
    load();
    const tick = setInterval(() => (now = Date.now()), 1000);
    const unlistenData = subscribe(onDataChanged(load));
    const unlistenClose = subscribe(listen('request-close', requestClose));
    return () => {
      clearInterval(tick);
      unlistenData();
      unlistenClose();
    };
  });

  // Expand again when the auto-hide setting is switched off.
  $effect(() => {
    if (!s.hideProjectsOnMouseExit && collapsed) setCollapsed(false);
  });

  function showError(e: unknown) {
    error = errorText(e);
    clearTimeout(errorTimer);
    errorTimer = setTimeout(() => (error = ''), 5000);
  }

  function onNotesInput() {
    clearTimeout(notesTimer);
    notesTimer = setTimeout(() => api.updateCurrentNotes(notes).catch(showError), 400);
  }

  function requestSwitch(project: Project, transfer = 0) {
    menu = null;
    const needsNotes = s.remindNotes && !notes.trim() && (!s.remindNotesOnlyWork || active?.isWork);
    if (needsNotes) {
      reminderNotes = '';
      dialog = { kind: 'notes', project, transfer };
      return;
    }
    doSwitch(project, transfer, notes);
  }

  async function doSwitch(project: Project, transfer: number, workNotes: string) {
    dialog = null;
    clearTimeout(notesTimer);
    try {
      await api.switchProject(project.id, workNotes, transfer);
      search = '';
    } catch (e) {
      showError(e);
    }
  }

  function openMenu(e: MouseEvent, project: Project) {
    e.preventDefault();
    menu = {
      project,
      x: Math.min(e.clientX, window.innerWidth - 210),
      y: Math.min(e.clientY, window.innerHeight - 230),
    };
  }

  /** Closes the context menu and runs `action` with its project (read before the menu is gone). */
  function fromMenu(action: (project: Project) => void) {
    const project = menu!.project;
    menu = null;
    action(project);
  }

  async function move(project: Project, delta: number) {
    const ids = projects.map((p) => p.id);
    const i = ids.indexOf(project.id);
    const j = i + delta;
    if (j < 0 || j >= ids.length) return;
    [ids[i], ids[j]] = [ids[j], ids[i]];
    try {
      await api.reorderProjects(ids);
    } catch (e) {
      showError(e);
    }
  }

  async function deleteProject(project: Project) {
    dialog = null;
    try {
      await api.deleteProject(project.id);
    } catch (e) {
      showError(e);
    }
  }

  function requestClose() {
    if (s.confirmClose) dialog = { kind: 'close' };
    else quit();
  }

  async function quit() {
    clearTimeout(notesTimer);
    await api.updateCurrentNotes(notes).catch(() => {});
    await api.quit();
  }

  async function setCollapsed(value: boolean) {
    if (value === collapsed) return;
    collapsed = value;
    // With the list on the left, keep the main panel in place by shifting the window.
    if (!s.displayProjectsRight) {
      const [factor, pos] = await Promise.all([win.scaleFactor(), win.outerPosition()]);
      const dx = Math.round(LIST_WIDTH * factor) * (value ? 1 : -1);
      await win.setPosition(new PhysicalPosition(pos.x + dx, pos.y));
    }
    await win.setSize(new LogicalSize(value ? MAIN_WIDTH : MAIN_WIDTH + LIST_WIDTH, HEIGHT));
  }

  function onMouseEnter() {
    clearTimeout(collapseTimer);
    if (collapsed) setCollapsed(false);
  }

  function onMouseLeave() {
    if (!s.hideProjectsOnMouseExit || dialog || menu) return;
    clearTimeout(collapseTimer);
    collapseTimer = setTimeout(() => setCollapsed(true), 400);
  }

  function startDrag(e: MouseEvent) {
    if (e.button !== 0 || (e.target as HTMLElement).closest('button, textarea, input')) return;
    win.startDragging();
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && filtered[0]) requestSwitch(filtered[0]);
    if (e.key === 'Escape') search = '';
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.ctrlKey && e.key === 'f') {
      e.preventDefault();
      searchInput?.focus();
    }
  }}
  onclick={() => (menu = null)}
/>
<svelte:document onmouseenter={onMouseEnter} onmouseleave={onMouseLeave} />

<div class="shell" class:left={!s.displayProjectsRight} style:--project={active?.color ?? 'var(--accent)'}>
  <section class="main" role="presentation" onmousedown={startDrag}>
    <header>
      <span class="brand"><span class="dot" style:background="var(--project)"></span>InTime</span>
      <div class="tools">
        <button class="icon-btn" title="Bericht" onclick={() => api.openWindow('report')}><Icon name="report" /></button>
        <button class="icon-btn" title="Einstellungen" onclick={() => api.openWindow('settings')}>
          <Icon name="settings" />
        </button>
        <button class="icon-btn" title="Minimieren" onclick={() => win.minimize()}><Icon name="minimize" /></button>
        <button class="icon-btn close" title="Beenden" onclick={requestClose}><Icon name="close" /></button>
      </div>
    </header>

    <div class="current">
      <div class="project-name" title={active?.description || active?.name}>{active?.name ?? '…'}</div>
      <div class="elapsed mono">{duration(elapsed, true)}</div>
      <div class="totals muted mono">
        Heute {duration(todayTotal, true)} <span class="sep">·</span> Arbeit {duration(todayWork, true)}
      </div>
    </div>

    <textarea
      class="notes"
      placeholder="Was machst du gerade?"
      bind:value={notes}
      oninput={onNotesInput}
      spellcheck="false"
    ></textarea>
  </section>

  {#if !collapsed}
    <section class="list">
      <div class="search">
        <Icon name="search" size={14} />
        <input
          type="search"
          placeholder="Projekt suchen…"
          bind:value={search}
          bind:this={searchInput}
          onkeydown={onSearchKey}
        />
      </div>
      <ul>
        {#each filtered as project (project.id)}
          <li>
            <button
              class="project"
              class:active={project.id === active?.id}
              style:--color={project.color}
              title={project.description || project.name}
              onclick={() => requestSwitch(project)}
              oncontextmenu={(e) => openMenu(e, project)}
            >
              <span class="dot" style:background={project.color}></span>
              <span class="name">{project.name}</span>
              {#if todayByProject.get(project.id)}
                <span class="time mono">{duration(todayByProject.get(project.id)!, true)}</span>
              {/if}
            </button>
          </li>
        {:else}
          <li class="empty muted">Kein Projekt gefunden.</li>
        {/each}
      </ul>
      <button class="add" onclick={() => api.openWindow('project')}><Icon name="plus" size={14} /> Neues Projekt</button>
    </section>
  {/if}

  {#if error}
    <div class="toast" role="alert">{error}</div>
  {/if}
</div>

{#if menu}
  <div class="menu" style:left="{menu.x}px" style:top="{menu.y}px" role="menu">
    <button role="menuitem" onclick={() => fromMenu((p) => requestSwitch(p))}>
      <Icon name="switch" size={14} /> Wechseln
    </button>
    <button
      role="menuitem"
      disabled={maxTransfer < 1}
      onclick={() =>
        fromMenu((p) => {
          transferMinutes = Math.min(5, maxTransfer);
          dialog = { kind: 'transfer', project: p };
        })}
    >
      <Icon name="clock" size={14} /> Wechseln mit Zeitübertrag…
    </button>
    <hr />
    <button role="menuitem" onclick={() => fromMenu((p) => api.openWindow('project', p.id).catch(showError))}>
      <Icon name="edit" size={14} /> Bearbeiten
    </button>
    <button role="menuitem" onclick={() => fromMenu((p) => move(p, -1))}>
      <Icon name="up" size={14} /> Nach oben
    </button>
    <button role="menuitem" onclick={() => fromMenu((p) => move(p, 1))}>
      <Icon name="down" size={14} /> Nach unten
    </button>
    <hr />
    <button
      role="menuitem"
      class="danger"
      disabled={menu.project.isDefault}
      onclick={() => fromMenu((p) => (dialog = { kind: 'delete', project: p }))}
    >
      <Icon name="trash" size={14} /> Löschen
    </button>
  </div>
{/if}

{#if dialog?.kind === 'notes'}
  {@const d = dialog}
  <Dialog title="Notiz zu „{active?.name}“" width={380} onclose={() => (dialog = null)}>
    <!-- svelte-ignore a11y_autofocus -->
    <textarea rows="3" placeholder="Was hast du gemacht?" bind:value={reminderNotes} autofocus></textarea>
    {#snippet actions()}
      <button class="btn" onclick={() => doSwitch(d.project, d.transfer, notes)}>Ohne Notiz</button>
      <button class="btn primary" onclick={() => doSwitch(d.project, d.transfer, reminderNotes)}>Wechseln</button>
    {/snippet}
  </Dialog>
{:else if dialog?.kind === 'transfer'}
  {@const d = dialog}
  <Dialog title="Zu „{d.project.name}“ wechseln" width={380} onclose={() => (dialog = null)}>
    <label class="field">
      Wie viele der letzten Minuten gehören schon zu „{d.project.name}“? (max. {maxTransfer})
      <!-- svelte-ignore a11y_autofocus -->
      <input type="number" min="0" max={maxTransfer} bind:value={transferMinutes} autofocus />
    </label>
    {#snippet actions()}
      <button class="btn" onclick={() => (dialog = null)}>Abbrechen</button>
      <button
        class="btn primary"
        onclick={() => requestSwitch(d.project, Math.max(0, Math.min(maxTransfer, transferMinutes || 0)))}
      >
        Wechseln
      </button>
    {/snippet}
  </Dialog>
{:else if dialog?.kind === 'delete'}
  {@const d = dialog}
  <Dialog title="Projekt löschen?" width={360} onclose={() => (dialog = null)}>
    <p class="muted">„{d.project.name}“ wird aus der Liste entfernt. Bereits erfasste Zeiten bleiben im Bericht erhalten.</p>
    {#snippet actions()}
      <button class="btn" onclick={() => (dialog = null)}>Abbrechen</button>
      <button class="btn danger" onclick={() => deleteProject(d.project)}>Löschen</button>
    {/snippet}
  </Dialog>
{:else if dialog?.kind === 'close'}
  <Dialog title="InTime beenden?" width={340} onclose={() => (dialog = null)}>
    <p class="muted">Die laufende Zeit wird gespeichert und die Erfassung beendet.</p>
    {#snippet actions()}
      <button class="btn" onclick={() => (dialog = null)}>Abbrechen</button>
      <button class="btn danger" onclick={quit}>Beenden</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .shell {
    display: flex;
    height: 100vh;
    border: 1px solid var(--border);
    border-top: 3px solid var(--project);
  }

  .shell.left {
    flex-direction: row-reverse;
  }

  .main {
    display: flex;
    flex-direction: column;
    flex: none;
    width: 298px;
    padding: 6px 12px 12px;
    gap: 8px;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--muted);
  }

  .tools {
    display: flex;
    gap: 2px;
  }

  .current {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .project-name {
    overflow: hidden;
    font-size: 18px;
    font-weight: 600;
    color: var(--project);
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .elapsed {
    font-size: 34px;
    font-weight: 300;
    line-height: 1.1;
    letter-spacing: 0.02em;
  }

  .totals {
    font-size: 12px;
  }

  .sep {
    margin: 0 4px;
  }

  .notes {
    flex: 1;
    min-height: 0;
    font-size: 13px;
  }

  .list {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    padding: 10px 8px 8px;
    gap: 6px;
    background: var(--surface);
    border-left: 1px solid var(--border);
  }

  .shell.left .list {
    border-left: none;
    border-right: 1px solid var(--border);
  }

  .search {
    position: relative;
    display: flex;
    align-items: center;
    color: var(--muted);
  }

  .search :global(svg) {
    position: absolute;
    left: 8px;
    pointer-events: none;
  }

  .search input {
    padding-left: 28px;
    font-size: 13px;
  }

  ul {
    flex: 1;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
  }

  .project {
    display: flex;
    align-items: center;
    width: 100%;
    gap: 8px;
    padding: 6px 8px;
    border: none;
    border-left: 3px solid transparent;
    border-radius: 4px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .project:hover {
    background: var(--hover-bg);
    color: var(--hover-fg);
  }

  .project.active {
    border-left-color: var(--color);
    background: var(--surface-2);
    font-weight: 600;
  }

  .name {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .time {
    font-size: 11px;
    color: var(--muted);
  }

  .empty {
    padding: 8px;
    font-size: 13px;
  }

  .add {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 6px;
    border: 1px dashed var(--border);
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    font-size: 13px;
    cursor: pointer;
  }

  .add:hover {
    background: var(--hover-bg);
    color: var(--hover-fg);
  }

  .menu {
    position: fixed;
    z-index: 40;
    display: flex;
    flex-direction: column;
    min-width: 200px;
    padding: 4px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.35);
  }

  .menu button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    border: none;
    border-radius: 4px;
    background: transparent;
    text-align: left;
    font-size: 13px;
    cursor: pointer;
  }

  .menu button:hover:not(:disabled) {
    background: var(--hover-bg);
    color: var(--hover-fg);
  }

  .menu button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .menu button.danger:not(:disabled) {
    color: var(--danger);
  }

  .menu hr {
    width: 100%;
    margin: 3px 0;
    border: none;
    border-top: 1px solid var(--border);
  }

  .toast {
    position: fixed;
    left: 10px;
    right: 10px;
    bottom: 10px;
    z-index: 60;
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--danger);
    color: #fff;
    font-size: 13px;
  }
</style>
