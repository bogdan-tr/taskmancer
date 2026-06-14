<script lang="ts">
  import { onMount } from "svelte";
  import "../styles/global.css";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import { initDisplay } from "$lib/displaySettings.svelte";
  import { refreshProjects } from "$lib/projects.svelte";
  import { refreshSettings } from "$lib/settings.svelte";
  import { initSidebar } from "$lib/sidebar.svelte";
  import { refreshTags } from "$lib/tags.svelte";
  import { initTheme } from "$lib/theme.svelte";

  let { children } = $props();

  initTheme();
  initSidebar();
  initDisplay();

  onMount(() => {
    void refreshProjects();
    void refreshTags();
    void refreshSettings();
  });
</script>

<div class="app-shell">
  <Sidebar />
  <div class="app-main">
    {@render children()}
  </div>
</div>

<style>
  .app-shell {
    display: flex;
    min-height: 100vh;
  }

  .app-main {
    flex: 1;
    min-width: 0;
  }
</style>
