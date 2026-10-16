<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import * as api from "$lib/api";
  import type { DoctorReport } from "$lib/api";

  let { onChange, onError }: { onChange: () => void; onError: (e: string) => void } = $props();

  let dirs = $state<string[]>([]);
  let report = $state<DoctorReport | null>(null);
  let busy = $state(false);

  onMount(async () => {
    try {
      dirs = (await api.getConfig()).extraScanDirs;
      report = await api.doctor();
    } catch (e) {
      onError(String(e));
    }
  });

  async function save(next: string[]) {
    try {
      await api.setExtraScanDirs(next);
      dirs = next;
      onChange();
    } catch (e) {
      onError(String(e));
    }
  }

  async function addDir() {
    const dir = await open({ directory: true, title: "Folder containing JDKs (or a JDK itself)" });
    if (typeof dir === "string" && !dirs.includes(dir)) save([...dirs, dir]);
  }

  async function rerunSetup() {
    busy = true;
    try {
      await api.runSetup();
      report = await api.doctor();
      onChange();
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }
</script>

<h2>Extra JDK folders</h2>
<p class="muted small">
  jman already scans the usual install locations, IntelliJ's <code>~/.jdks</code>, Gradle, SDKMAN and its own downloads.
</p>
{#if dirs.length}
  <ul>
    {#each dirs as dir}
      <li>
        <code>{dir}</code>
        <button class="link" onclick={() => save(dirs.filter((d) => d !== dir))}>remove</button>
      </li>
    {/each}
  </ul>
{/if}
<button class="primary" onclick={addDir}>Add folder…</button>

<h2>Environment</h2>
{#if report}
  <dl>
    <dt>JAVA_HOME</dt>
    <dd><code>{report.jmanDir}{report.jmanDir.includes("\\") ? "\\" : "/"}current</code></dd>
    <dt>Selected</dt>
    <dd>{report.current ? `${report.current.id} (${report.current.version})` : "none"}</dd>
  </dl>
  {#if report.setup.issues.length}
    <ul class="issues">
      {#each report.setup.issues as issue}
        <li>{issue}</li>
      {/each}
    </ul>
  {:else}
    <p class="ok">Setup is complete.</p>
  {/if}
{/if}
<button class="primary" onclick={rerunSetup} disabled={busy}>{busy ? "Running…" : "Re-run setup"}</button>

<h2>IDEs</h2>
<p class="small">
  Point your IDE's JDK at the <code>JAVA_HOME</code> path above, not at a specific JDK. IntelliJ caches the
  version it detected, and Gradle/Maven daemons keep the JDK they started with, so after a major-version
  switch reload the SDK or stop the daemons (<code>gradle --stop</code>).
</p>

<style>
  h2 {
    font-size: 14px;
    margin: 18px 0 6px;
  }
  h2:first-child {
    margin-top: 4px;
  }
  .small {
    font-size: 12px;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0 0 8px;
  }
  li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 4px 0;
    word-break: break-all;
  }
  .issues li {
    color: var(--danger);
    font-size: 13px;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 10px;
    margin: 0 0 8px;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    word-break: break-all;
  }
  .ok {
    color: var(--accent);
  }
</style>
