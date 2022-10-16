<script>
  import Greet from "./lib/Greet.svelte";
  import pack from "../package.json";
  import { invoke } from "@tauri-apps/api/tauri";

  let installedJDKS;
  let selectedJDK = "";
  (async () => {
    installedJDKS = Array.from(await invoke("get_installed_jdks"));
    console.log(installedJDKS);

    selectedJDK = await invoke("get_current_jdk");
    console.log(selectedJDK);

    console.log(selectedJDK.includes("jdk1.8.0_341"));
  })();
</script>

<body>
  <fieldset>
    <legend align="left">jman v{pack.version}</legend>

    <!-- <p>Currently running: <b>{selectedJDK}</b></p> -->
    <p>Installed JDKs detected:</p>
    {#if installedJDKS}
      {#each installedJDKS as jdk}
        {#if /jdk(|-)([0-9._]+)/.exec(selectedJDK)[2]}
          <p><b> >{jdk}</b></p>
        {:else}
          <p>{jdk}</p>
        {/if}
      {/each}
    {/if}
  </fieldset>
</body>

<style>
  @import url("https://fonts.googleapis.com/css2?family=Rubik&display=swap");
  * {
    font-family: Rubik, sans-serif;
  }

  b {
    color: lime;
  }

  body {
    height: 92.5vh;
  }

  fieldset {
    height: 100%;
  }

  legend {
    font-size: 2rem;
  }
</style>
