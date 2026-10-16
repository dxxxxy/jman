<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import * as api from "$lib/api";
  import type { Jdk } from "$lib/api";

  let {
    jdks,
    current,
    onChange,
    onError,
  }: { jdks: Jdk[]; current: Jdk | null; onChange: () => void; onError: (e: string) => void } = $props();

  let switching = $state<string | null>(null);

  const isCurrent = (jdk: Jdk) => current?.id === jdk.id;

  async function select(jdk: Jdk) {
    if (switching || isCurrent(jdk)) return;
    switching = jdk.id;
    try {
      await api.useJdk(jdk.id);
      onChange();
    } catch (e) {
      onError(String(e));
    } finally {
      switching = null;
    }
  }

  async function remove(jdk: Jdk) {
    const ok = await ask(`Delete ${jdk.id}?\n\nThis removes ${jdk.home}`, { title: "jman", kind: "warning" });
    if (!ok) return;
    try {
      await api.removeJdk(jdk.id);
      onChange();
    } catch (e) {
      onError(String(e));
    }
  }

  // up/down arrows switch to the previous/next JDK, like jman v1
  function onKeydown(e: KeyboardEvent) {
    if (e.key !== "ArrowUp" && e.key !== "ArrowDown") return;
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;
    e.preventDefault();
    const index = jdks.findIndex(isCurrent);
    const next = jdks[index + (e.key === "ArrowUp" ? -1 : 1)] ?? (index === -1 ? jdks[0] : undefined);
    if (next) select(next);
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if jdks.length === 0}
  <p class="muted">No JDKs found. Download one, or add a folder in Settings.</p>
{:else}
  <ul>
    {#each jdks as jdk (jdk.id)}
      <li class:current={isCurrent(jdk)}>
        <button class="item" onclick={() => select(jdk)} title={jdk.home}>
          <span class="marker">{isCurrent(jdk) ? "›" : ""}</span>
          <span class="name">{jdk.id}</span>
          <span class="meta">
            {jdk.version}
            {#if jdk.vendor}· {jdk.vendor}{/if}
          </span>
          {#if switching === jdk.id}<span class="muted">switching…</span>{/if}
        </button>
        {#if jdk.managed && !isCurrent(jdk)}
          <button class="remove" title="Delete this JDK" aria-label="Delete {jdk.id}" onclick={() => remove(jdk)}>×</button>
        {/if}
      </li>
    {/each}
  </ul>
  <p class="hint muted">Click a JDK or use ↑ / ↓ to switch.</p>
{/if}

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  li {
    display: flex;
    align-items: center;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  li.current {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .item {
    flex: 1;
    display: grid;
    grid-template-columns: 14px 1fr auto;
    grid-template-areas: "marker name status" "marker meta status";
    align-items: center;
    column-gap: 6px;
    text-align: left;
    background: none;
    border: 0;
    padding: 8px 10px;
  }
  .marker {
    grid-area: marker;
    color: var(--accent);
    font-weight: 700;
    font-size: 18px;
  }
  .name {
    grid-area: name;
    font-weight: 600;
  }
  li.current .name {
    color: var(--accent);
  }
  .meta {
    grid-area: meta;
    color: var(--muted);
    font-size: 12px;
  }
  .remove {
    background: none;
    border: 0;
    color: var(--muted);
    font-size: 18px;
    padding: 0 12px;
  }
  .remove:hover {
    color: var(--danger);
  }
  .hint {
    font-size: 12px;
    text-align: center;
  }
</style>
