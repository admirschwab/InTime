<script lang="ts">
  import { initSettings, settings } from './lib/settings.svelte';
  import MainView from './views/MainView.svelte';
  import ReportView from './views/ReportView.svelte';
  import SettingsView from './views/SettingsView.svelte';
  import ProjectView from './views/ProjectView.svelte';

  // Set by the backend for secondary windows (see windows.rs); the main window has none.
  const ctx: { view: string; id: number | null } = (window as any).__INTIME__ ?? { view: 'main', id: null };

  initSettings();

  // Prevent the WebView's own context menu and page reloads from feeling like a browser.
  window.addEventListener('contextmenu', (e) => {
    if (!(e.target as HTMLElement).closest('input, textarea')) e.preventDefault();
  });
</script>

{#if settings.value}
  {#if ctx.view === 'report'}
    <ReportView />
  {:else if ctx.view === 'settings'}
    <SettingsView />
  {:else if ctx.view === 'project'}
    <ProjectView id={ctx.id} />
  {:else}
    <MainView />
  {/if}
{/if}
