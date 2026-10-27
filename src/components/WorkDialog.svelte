<script lang="ts">
  import { untrack } from 'svelte';
  import { api, errorText, type Project, type Work } from '../lib/api';
  import { addDays, dateKey } from '../lib/format';
  import Dialog from './Dialog.svelte';

  let {
    work,
    date,
    projects,
    onclose,
  }: {
    /** `null` creates a new entry. */
    work: Work | null;
    /** Default day for a new entry. */
    date: Date;
    projects: Project[];
    onclose: () => void;
  } = $props();

  // The form is initialised once from the props it was opened with.
  const { initial, defaultProject, defaultDay } = untrack(() => ({
    initial: work,
    defaultProject: projects.find((p) => p.isEnabled && p.isWork)?.id ?? projects[0]?.id,
    defaultDay: dateKey(date),
  }));
  let projectId = $state(initial?.projectId ?? defaultProject);
  let day = $state(initial ? initial.startTime.slice(0, 10) : defaultDay);
  let start = $state(initial ? initial.startTime.slice(11, 19) : '09:00:00');
  let end = $state(initial ? initial.endTime.slice(11, 19) : '10:00:00');
  let notes = $state(initial?.notes ?? '');
  let error = $state('');

  // Disabled projects are only offered if the entry already belongs to one.
  const choices = $derived(projects.filter((p) => p.isEnabled || p.id === initial?.projectId));

  const withSeconds = (t: string) => (t.length === 5 ? `${t}:00` : t);

  async function save(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    if (!start || !end || !day) {
      error = 'Bitte Datum, Beginn und Ende angeben.';
      return;
    }
    // An end before the start means the entry runs past midnight.
    const endDay = withSeconds(end) <= withSeconds(start) ? dateKey(addDays(new Date(`${day}T00:00:00`), 1)) : day;
    const input = {
      projectId,
      startTime: `${day}T${withSeconds(start)}`,
      endTime: `${endDay}T${withSeconds(end)}`,
      notes,
    };
    try {
      if (initial) await api.updateWork(initial.id, input);
      else await api.createWork(input);
      onclose();
    } catch (err) {
      error = errorText(err);
    }
  }
</script>

<Dialog title={initial ? 'Eintrag bearbeiten' : 'Eintrag hinzufügen'} width={420} {onclose}>
  <form id="work-form" onsubmit={save}>
    <label class="field">
      Projekt
      <select bind:value={projectId}>
        {#each choices as p (p.id)}
          <option value={p.id}>{p.name}{p.isEnabled ? '' : ' (gelöscht)'}</option>
        {/each}
      </select>
    </label>
    <div class="row">
      <label class="field">Datum <input type="date" bind:value={day} /></label>
      <label class="field">Beginn <input type="time" step="1" bind:value={start} /></label>
      <label class="field">Ende <input type="time" step="1" bind:value={end} /></label>
    </div>
    <label class="field">
      Notizen
      <textarea rows="4" bind:value={notes}></textarea>
    </label>
    {#if error}<p class="error">{error}</p>{/if}
  </form>
  {#snippet actions()}
    <button class="btn" type="button" onclick={onclose}>Abbrechen</button>
    <button class="btn primary" type="submit" form="work-form">Speichern</button>
  {/snippet}
</Dialog>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .row {
    display: grid;
    grid-template-columns: 1.3fr 1fr 1fr;
    gap: 8px;
  }

  .error {
    margin: 0;
  }
</style>
