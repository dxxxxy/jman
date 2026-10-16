<script lang="ts">
  import * as api from "$lib/api";
  import type { SetupStatus } from "$lib/api";

  let { status, onDone }: { status: SetupStatus; onDone: () => void } = $props();

  let busy = $state(false);
  let message = $state("");

  async function setup() {
    busy = true;
    message = "";
    try {
      const result = await api.runSetup();
      message = result.configured
        ? "Done. Open a new terminal and restart your IDE once. After that, switching takes effect immediately."
        : result.issues.join("; ");
      onDone();
    } catch (e) {
      message = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="banner">
  <strong>One-time setup</strong>
  <p>
    jman points <code>JAVA_HOME</code> and <code>PATH</code> at one fixed folder. After that, you
    can switch JDKs without restarting terminals or IDEs.
  </p>
  {#if status.issues.length}
    <ul>
      {#each status.issues as issue}
        <li>{issue}</li>
      {/each}
    </ul>
  {/if}
  <div class="row">
    <button class="primary" onclick={setup} disabled={busy}>{busy ? "Setting up…" : "Set up"}</button>
    {#if status.needsElevation}
      <span class="muted">Windows will ask for administrator rights once.</span>
    {/if}
  </div>
  {#if message}
    <p class="message">{message}</p>
  {/if}
</div>

<style>
  .banner {
    background: var(--warn-bg);
    border: 1px solid var(--warn-border);
    border-radius: 8px;
    padding: 10px 12px;
  }
  p {
    margin: 4px 0 8px;
  }
  ul {
    margin: 0 0 8px;
    padding-left: 18px;
    color: var(--muted);
    font-size: 13px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
  }
  .message {
    margin: 8px 0 0;
    user-select: text;
  }
</style>
