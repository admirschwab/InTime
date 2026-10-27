<script lang="ts" module>
  export interface Segment {
    start: number;
    end: number;
    color: string;
    label: string;
  }
</script>

<script lang="ts">
  let { segments }: { segments: Segment[] } = $props();

  const HOUR = 3_600_000;

  // Span from the full hour before the first entry to the full hour after the last.
  const range = $derived.by(() => {
    if (segments.length === 0) return null;
    const first = new Date(Math.min(...segments.map((s) => s.start)));
    first.setMinutes(0, 0, 0);
    const from = first.getTime();
    const to = Math.max(from + HOUR, Math.ceil(Math.max(...segments.map((s) => s.end)) / HOUR) * HOUR);
    return { from, to };
  });

  const hours = $derived.by(() => {
    if (!range) return [];
    const count = Math.round((range.to - range.from) / HOUR);
    const step = count > 12 ? 2 : 1;
    const result: { left: number; label: string }[] = [];
    for (let i = 0; i <= count; i += step) {
      const t = range.from + i * HOUR;
      result.push({ left: ((t - range.from) / (range.to - range.from)) * 100, label: `${new Date(t).getHours()}` });
    }
    return result;
  });

  const pct = (t: number) => (range ? ((t - range.from) / (range.to - range.from)) * 100 : 0);
</script>

{#if range}
  <div class="timeline">
    <div class="bar">
      {#each segments as s, i (i)}
        <div
          class="segment"
          style:left="{pct(s.start)}%"
          style:width="max(2px, {pct(s.end) - pct(s.start)}%)"
          style:background={s.color}
          title={s.label}
        ></div>
      {/each}
    </div>
    <div class="scale">
      {#each hours as h (h.left)}
        <span style:left="{h.left}%">{h.label}</span>
      {/each}
    </div>
  </div>
{/if}

<style>
  .timeline {
    padding: 0 8px;
  }

  .bar {
    position: relative;
    height: 26px;
    background: var(--surface-2);
    border-radius: 6px;
    overflow: hidden;
  }

  .segment {
    position: absolute;
    top: 0;
    bottom: 0;
  }

  .scale {
    position: relative;
    height: 18px;
    font-size: 11px;
    color: var(--muted);
  }

  .scale span {
    position: absolute;
    top: 3px;
    transform: translateX(-50%);
  }
</style>
