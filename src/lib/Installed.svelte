<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import * as api from "$lib/api";
  import type { Jdk, Removal } from "$lib/api";

  let {
    jdks,
    current,
    onChange,
    onError,
  }: { jdks: Jdk[]; current: Jdk | null; onChange: () => void; onError: (e: string) => void } = $props();

  let switching = $state<string | null>(null);
  let removing = $state<string | null>(null);
  const busy = $derived(switching !== null || removing !== null);

  const isCurrent = (jdk: Jdk) => current?.id === jdk.id;

  async function select(jdk: Jdk) {
    if (busy || isCurrent(jdk)) return;
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

  function confirmText(jdk: Jdk, plan: Removal): string {
    switch (plan.kind) {
      case "delete":
        return `Delete ${jdk.id}?\n\nThis deletes the folder ${plan.path}` +
          (plan.needsAdmin ? "\n\nAdministrator rights are needed." : "");
      case "uninstaller":
        return `Remove ${jdk.id}?\n\nThis runs the ${plan.name} uninstaller.`;
      case "package": {
        const others = plan.affected.filter((p) => p !== plan.package);
        return `Remove ${jdk.id}?\n\nThis removes the ${plan.package} package with ${plan.manager}.` +
          (others.length ? `\n\nThese packages are removed too:\n${others.join("\n")}` : "");
      }
    }
  }

  async function remove(jdk: Jdk) {
    if (busy) return;
    try {
      const plan = await api.removalPlan(jdk.id);
      const ok = await ask(confirmText(jdk, plan), { title: "jman", kind: "warning", okLabel: "Remove", cancelLabel: "Cancel" });
      if (!ok) return;
      removing = jdk.id;
      await api.removeJdk(jdk.id);
      onChange();
    } catch (e) {
      onError(String(e));
    } finally {
      removing = null;
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
      <li class:current={isCurrent(jdk)} class:removing={removing === jdk.id}>
        <button class="item" onclick={() => select(jdk)} disabled={busy && switching !== jdk.id}>
          <span class="marker">{isCurrent(jdk) ? "›" : ""}</span>
          <span class="name">{jdk.id}</span>
          <span class="meta">
            {jdk.version}
            {#if jdk.vendor}· {jdk.vendor}{/if}
            <span class="source" class:managed={jdk.managed}>{jdk.source}</span>
          </span>
          {#if switching === jdk.id}<span class="status muted">switching…</span>{/if}
          {#if removing === jdk.id}<span class="status muted">removing…</span>{/if}
        </button>
        {#if !isCurrent(jdk)}
          <button class="remove" title="Remove this JDK" aria-label="Remove {jdk.id}" disabled={busy} onclick={() => remove(jdk)}>×</button>
        {/if}
        <code class="path" title={jdk.home}>{jdk.home}</code>
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
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  li.current {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  li.removing {
    opacity: 0.6;
  }
  .item {
    display: grid;
    grid-template-columns: 14px 1fr auto;
    grid-template-areas: "marker name status" "marker meta status";
    align-items: center;
    column-gap: 6px;
    text-align: left;
    background: none;
    border: 0;
    padding: 8px 10px 2px;
  }
  .item:disabled {
    cursor: default;
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
  .status {
    grid-area: status;
    font-size: 12px;
  }
  .source {
    margin-left: 4px;
    padding: 0 6px;
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: 11px;
    white-space: nowrap;
  }
  .source.managed {
    color: var(--accent);
    border-color: var(--accent);
  }
  .path {
    grid-column: 1 / -1;
    padding: 0 10px 8px 30px;
    color: var(--muted);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .remove {
    align-self: start;
    background: none;
    border: 0;
    color: var(--muted);
    font-size: 18px;
    padding: 6px 12px;
  }
  .remove:hover:not(:disabled) {
    color: var(--danger);
  }
  .remove:disabled {
    cursor: default;
    opacity: 0.4;
  }
  .hint {
    font-size: 12px;
    text-align: center;
  }
</style>
