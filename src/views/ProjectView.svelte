<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { api, errorText, type Project } from '../lib/api';

  let { id }: { id: number | null } = $props();

  const PRESETS = [
    '#e5534b', '#f0883e', '#d4a72c', '#57ab5a', '#2ea0a0', '#4c8bf5',
    '#8f6ff5', '#d062b8', '#8a8f98', '#6e4b3a', '#1f7a4d', '#24477a',
  ];

  let name = $state('');
  let description = $state('');
  let color = $state(PRESETS[Math.floor(Math.random() * PRESETS.length)]);
  let isWork = $state(true);
  let isDefault = $state(false);
  let existing = $state<Project | null>(null);
  let error = $state('');
  let nameInput: HTMLInputElement | undefined = $state();

  $effect(() => {
    if (id === null) {
      nameInput?.focus();
      return;
    }
    api.projects(true).then((list) => {
      const p = list.find((p) => p.id === id);
      if (!p) {
        error = 'Projekt nicht gefunden.';
        return;
      }
      existing = p;
      ({ name, description, color, isWork, isDefault } = p);
      nameInput?.focus();
    });
  });

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    const input = { name, description, color, isWork, isDefault };
    try {
      if (id === null) await api.createProject(input);
      else await api.updateProject(id, input);
      await getCurrentWindow().close();
    } catch (err) {
      error = errorText(err);
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && getCurrentWindow().close()} />

<form class="project" onsubmit={submit}>
  <div class="preview" style:--color={color}>
    <span class="dot big"></span>
    <span class="name">{name || 'Neues Projekt'}</span>
  </div>

  <label class="field">
    Name
    <input type="text" bind:value={name} bind:this={nameInput} maxlength="80" />
  </label>

  <label class="field">
    Beschreibung
    <textarea rows="3" bind:value={description}></textarea>
  </label>

  <div class="field">
    Farbe
    <div class="swatches">
      {#each PRESETS as c (c)}
        <button
          type="button"
          class="swatch"
          class:selected={c === color}
          style:background={c}
          title={c}
          aria-label="Farbe {c}"
          onclick={() => (color = c)}
        ></button>
      {/each}
      <input type="color" bind:value={color} title="Eigene Farbe" />
    </div>
  </div>

  <label class="check">
    <input type="checkbox" bind:checked={isWork} />
    Zählt als Arbeitszeit
  </label>
  <label class="check" title={existing?.isDefault ? 'Markiere ein anderes Projekt als Standard.' : ''}>
    <input type="checkbox" bind:checked={isDefault} disabled={existing?.isDefault} />
    Standardprojekt (wird beim Start ausgewählt)
  </label>

  {#if error}<p class="error">{error}</p>{/if}

  <div class="actions">
    <button type="button" class="btn" onclick={() => getCurrentWindow().close()}>Abbrechen</button>
    <button type="submit" class="btn primary">{id === null ? 'Anlegen' : 'Speichern'}</button>
  </div>
</form>

<style>
  .project {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 18px 20px;
    gap: 14px;
    overflow-y: auto;
  }

  .preview {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-left: 4px solid var(--color);
    border-radius: 6px;
    background: var(--surface);
  }

  .dot.big {
    width: 14px;
    height: 14px;
    background: var(--color);
  }

  .preview .name {
    overflow: hidden;
    font-size: 16px;
    font-weight: 600;
    color: var(--color);
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
  }

  .swatches {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .swatch {
    width: 24px;
    height: 24px;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 50%;
    cursor: pointer;
  }

  .swatch.selected {
    border-color: var(--fg);
  }

  input[type='color'] {
    width: 32px;
    height: 24px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: none;
    cursor: pointer;
  }

  .error {
    margin: 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: auto;
  }
</style>
