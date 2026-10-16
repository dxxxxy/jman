<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "$lib/api";
  import type { Distribution, MajorVersion, Package, Progress } from "$lib/api";

  let { onInstalled, onError }: { onInstalled: () => void; onError: (e: string) => void } = $props();

  let distributions = $state<Distribution[]>([]);
  let versions = $state<MajorVersion[]>([]);
  let distribution = $state("temurin");
  let major = $state(0);
  let pkg = $state<Package | null | undefined>(undefined);
  let searching = $state(false);
  let progress = $state<Progress | null>(null);

  onMount(async () => {
    try {
      ({ distributions, versions } = await api.downloadOptions());
      major = versions.find((v) => v.termOfSupport.toLowerCase() === "lts")?.majorVersion ?? versions[0]?.majorVersion;
    } catch (e) {
      onError(`Could not reach the foojay API: ${e}`);
    }
  });

  // look up the package whenever the selection changes
  $effect(() => {
    const [d, m] = [distribution, major];
    if (!m) return;
    pkg = undefined;
    searching = true;
    api
      .findPackage(d, m)
      .then((p) => {
        if (d === distribution && m === major) pkg = p;
      })
      .catch((e) => onError(String(e)))
      .finally(() => (searching = false));
  });

  async function install() {
    progress = { phase: "downloading", done: 0, total: pkg?.size ?? 0 };
    try {
      await api.installJdk(distribution, major, (p) => (progress = p));
      onInstalled();
    } catch (e) {
      onError(String(e));
    } finally {
      progress = null;
    }
  }

  const percent = $derived(
    progress?.phase === "downloading" && progress.total ? Math.round((progress.done / progress.total) * 100) : null,
  );
</script>

<div class="form">
  <label>
    Distribution
    <select bind:value={distribution} disabled={!!progress}>
      {#each distributions as d}
        <option value={d.id}>{d.name}</option>
      {/each}
    </select>
  </label>
  <label>
    Version
    <select bind:value={major} disabled={!!progress}>
      {#each versions as v}
        <option value={v.majorVersion}>{v.majorVersion}{v.termOfSupport.toLowerCase() === "lts" ? " (LTS)" : ""}</option>
      {/each}
    </select>
  </label>
</div>

<div class="result">
  {#if searching}
    <p class="muted">Looking up the latest build…</p>
  {:else if pkg}
    <p><strong>{pkg.distribution} {pkg.javaVersion}</strong> <span class="muted">· {api.formatMb(pkg.size)}</span></p>
    <p class="muted file">{pkg.filename}</p>
    {#if progress}
      <div class="bar"><div style:width="{percent ?? 100}%" class:indeterminate={percent === null}></div></div>
      <p class="muted">
        {#if progress.phase === "downloading"}
          Downloading {percent ?? 0}% ({api.formatMb(progress.done)})
        {:else if progress.phase === "verifying"}
          Verifying checksum…
        {:else}
          Extracting…
        {/if}
      </p>
    {:else}
      <button class="primary" onclick={install}>Install</button>
    {/if}
  {:else if pkg === null}
    <p class="muted">No build of this version for your platform.</p>
  {/if}
</div>

<p class="muted note">JDKs are installed into <code>~/.jman/jdks</code>. Builds come from the foojay Disco API.</p>

<style>
  .form {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--muted);
  }
  .result {
    margin-top: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    min-height: 92px;
  }
  .result p {
    margin: 0 0 6px;
  }
  .file {
    font-size: 12px;
    word-break: break-all;
  }
  .bar {
    height: 6px;
    background: var(--border);
    border-radius: 3px;
    overflow: hidden;
    margin: 10px 0 6px;
  }
  .bar div {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s;
  }
  .bar .indeterminate {
    animation: pulse 1s ease-in-out infinite alternate;
  }
  @keyframes pulse {
    from {
      opacity: 0.4;
    }
  }
  .note {
    font-size: 12px;
  }
</style>
