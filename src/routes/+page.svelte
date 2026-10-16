<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import * as api from "$lib/api";
  import type { Jdk, SetupStatus } from "$lib/api";
  import Installed from "$lib/Installed.svelte";
  import Download from "$lib/Download.svelte";
  import Settings from "$lib/Settings.svelte";
  import SetupBanner from "$lib/SetupBanner.svelte";

  type Tab = "installed" | "download" | "settings";

  let tab = $state<Tab>("installed");
  let jdks = $state<Jdk[]>([]);
  let current = $state<Jdk | null>(null);
  let status = $state<SetupStatus | null>(null);
  let version = $state("");
  let error = $state("");

  async function refresh() {
    try {
      [jdks, current, status] = await Promise.all([api.listJdks(), api.currentJdk(), api.setupStatus()]);
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    refresh();
    getVersion().then((v) => (version = v));
    const unlisteners = [
      listen("jdks-changed", refresh),
      getCurrentWindow().onFocusChanged(({ payload: focused }) => focused && refresh()),
    ];
    return () => unlisteners.forEach((u) => u.then((f) => f()));
  });
</script>

<main>
  <header>
    <h1>jman <span class="version">v{version}</span></h1>
    {#if current}
      <span class="pill" title={current.home}>java {current.version}</span>
    {/if}
  </header>

  {#if status && !status.configured}
    <SetupBanner {status} onDone={refresh} />
  {/if}

  <nav>
    <button class:active={tab === "installed"} onclick={() => (tab = "installed")}>Installed</button>
    <button class:active={tab === "download"} onclick={() => (tab = "download")}>Download</button>
    <button class:active={tab === "settings"} onclick={() => (tab = "settings")}>Settings</button>
  </nav>

  {#if error}
    <p class="error" role="alert">{error} <button class="link" onclick={() => (error = "")}>dismiss</button></p>
  {/if}

  <section>
    {#if tab === "installed"}
      <Installed {jdks} {current} onChange={refresh} onError={(e) => (error = e)} />
    {:else if tab === "download"}
      <Download onInstalled={() => { tab = "installed"; refresh(); }} onError={(e) => (error = e)} />
    {:else}
      <Settings onChange={refresh} onError={(e) => (error = e)} />
    {/if}
  </section>
</main>

<style>
  :global(:root) {
    --bg: #f6f6f4;
    --surface: #ffffff;
    --text: #1c1c1e;
    --muted: #6b6b70;
    --border: #e2e2e0;
    --accent: #2f9e44;
    --accent-soft: #e6f4e9;
    --danger: #d9363e;
    --warn-bg: #fff6db;
    --warn-border: #f0d78c;
    color-scheme: light dark;
  }
  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg: #1b1b1d;
      --surface: #242427;
      --text: #ececee;
      --muted: #9a9aa0;
      --border: #34343a;
      --accent: #5bd16f;
      --accent-soft: #1f3324;
      --danger: #ff6b70;
      --warn-bg: #3a3220;
      --warn-border: #6b5a2a;
    }
  }
  :global(*) {
    box-sizing: border-box;
  }
  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--text);
    font: 14px/1.45 system-ui, -apple-system, "Segoe UI", Rubik, sans-serif;
    user-select: none;
  }
  :global(button) {
    font: inherit;
    color: inherit;
    cursor: pointer;
  }
  :global(button.primary) {
    background: var(--accent);
    color: #fff;
    border: 0;
    border-radius: 6px;
    padding: 6px 14px;
    font-weight: 600;
  }
  :global(button.primary:disabled) {
    opacity: 0.55;
    cursor: default;
  }
  :global(button.link) {
    background: none;
    border: 0;
    padding: 0;
    color: var(--muted);
    text-decoration: underline;
  }
  :global(select) {
    font: inherit;
    color: inherit;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 5px 8px;
  }
  :global(.muted) {
    color: var(--muted);
  }
  :global(code) {
    font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
    font-size: 12px;
    user-select: text;
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 16px;
    gap: 12px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h1 {
    margin: 0;
    font-size: 22px;
  }
  .version {
    font-size: 13px;
    font-weight: 400;
    color: var(--muted);
  }
  .pill {
    background: var(--accent-soft);
    color: var(--accent);
    border-radius: 999px;
    padding: 2px 10px;
    font-weight: 600;
    font-size: 13px;
  }
  nav {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border);
  }
  nav button {
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    padding: 6px 10px;
    color: var(--muted);
  }
  nav button.active {
    color: var(--text);
    border-bottom-color: var(--accent);
    font-weight: 600;
  }
  section {
    flex: 1;
    overflow-y: auto;
  }
  .error {
    margin: 0;
    color: var(--danger);
    user-select: text;
  }
</style>
