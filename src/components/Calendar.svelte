<script lang="ts">
  import Icon from './Icon.svelte';

  let {
    selected,
    year,
    month,
    marked,
    onselect,
    onmonth,
  }: {
    selected: Date;
    /** Displayed month, `month` is 1-based. */
    year: number;
    month: number;
    /** Days of the displayed month that have tracked work. */
    marked: Set<number>;
    onselect: (date: Date) => void;
    onmonth: (year: number, month: number) => void;
  } = $props();

  const WEEKDAYS = ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'];

  const title = $derived(
    new Date(year, month - 1, 1).toLocaleDateString('de-DE', { month: 'long', year: 'numeric' }),
  );

  // Leading blanks so weeks start on Monday.
  const cells = $derived.by(() => {
    const offset = (new Date(year, month - 1, 1).getDay() + 6) % 7;
    const days = new Date(year, month, 0).getDate();
    return [...Array(offset).fill(null), ...Array.from({ length: days }, (_, i) => i + 1)] as (number | null)[];
  });

  const today = new Date();
  const isSame = (d: Date, day: number) => d.getFullYear() === year && d.getMonth() === month - 1 && d.getDate() === day;

  function shift(delta: number) {
    const d = new Date(year, month - 1 + delta, 1);
    onmonth(d.getFullYear(), d.getMonth() + 1);
  }
</script>

<div class="calendar">
  <div class="head">
    <button class="icon-btn" onclick={() => shift(-1)} title="Vorheriger Monat"><Icon name="left" /></button>
    <span class="title">{title}</span>
    <button class="icon-btn" onclick={() => shift(1)} title="Nächster Monat"><Icon name="right" /></button>
  </div>
  <div class="grid">
    {#each WEEKDAYS as wd (wd)}
      <span class="wd">{wd}</span>
    {/each}
    {#each cells as day, i (i)}
      {#if day === null}
        <span></span>
      {:else}
        <button
          class="day"
          class:marked={marked.has(day)}
          class:selected={isSame(selected, day)}
          class:today={isSame(today, day)}
          onclick={() => onselect(new Date(year, month - 1, day))}
        >
          {day}
        </button>
      {/if}
    {/each}
  </div>
</div>

<style>
  .calendar {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .title {
    font-weight: 600;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
    text-align: center;
  }

  .wd {
    font-size: 11px;
    color: var(--muted);
    padding: 2px 0;
  }

  .day {
    position: relative;
    aspect-ratio: 1;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    font-size: 13px;
  }

  .day:hover {
    background: var(--hover-bg);
    color: var(--hover-fg);
  }

  .day.marked {
    color: var(--fg);
    font-weight: 600;
  }

  .day.marked::after {
    content: '';
    position: absolute;
    left: 50%;
    bottom: 3px;
    width: 4px;
    height: 4px;
    margin-left: -2px;
    border-radius: 50%;
    background: var(--accent);
  }

  .day.today {
    border-color: var(--border);
  }

  .day.selected {
    background: var(--accent);
    color: #fff;
  }

  .day.selected::after {
    background: #fff;
  }
</style>
