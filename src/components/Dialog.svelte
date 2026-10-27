<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    title,
    width = 360,
    onclose,
    children,
    actions,
  }: { title: string; width?: number; onclose: () => void; children: Snippet; actions?: Snippet } = $props();
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="backdrop" role="presentation" onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label={title} style:max-width="{width}px">
    <h3>{title}</h3>
    <div class="body">{@render children()}</div>
    {#if actions}
      <div class="actions">{@render actions()}</div>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 12px;
    background: rgb(0 0 0 / 0.45);
  }

  .dialog {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
    max-height: 100%;
    padding: 16px;
    overflow: auto;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.4);
  }

  h3 {
    font-size: 15px;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
