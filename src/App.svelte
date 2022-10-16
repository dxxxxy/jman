<script>
  // @ts-nocheck

  import Loader from "./lib/Loader.svelte";
  import NotElevated from "./lib/NotElevated.svelte";
  import pack from "../package.json";
  import { invoke } from "@tauri-apps/api/tauri";
  import { listen } from "@tauri-apps/api/event";

  let installedJDKS;
  let loading = false;
  let selectedJDK = "";
  let loader;
  let notElevated;

  (async () => {
    installedJDKS = Array.from(await invoke("get_installed_jdks"));
    selectedJDK = await invoke("get_current_jdk");
  })();

  listen("log", (event) => {
    loader.push(event.payload);
  });

  const getVersionFromFolderName = (jdk) => {
    const results = /jdk(|-)([0-9._]+)/.exec(jdk);
    return results[2];
  };

  //on up/down arrow, select the next/previous JDK in the ul
  document.addEventListener("keyup", async (e) => {
    if (loading) return;

    //get position of selected element
    const selected =
      document.querySelector("li.selected") ||
      document.querySelectorAll("li")[0];
    const ul = document.querySelector("ul");
    Array.prototype.indexOf.call(ul.childNodes, selected);
    let newJDK;

    if (e.key === "ArrowUp") {
      //select the previous element
      newJDK = selected.previousElementSibling;

      if (newJDK) {
        newJDK.classList.add("selected");
        selected.classList.remove("selected");
      }
    } else if (e.key === "ArrowDown") {
      //select the next element
      newJDK = selected.nextElementSibling;

      if (newJDK) {
        newJDK.classList.add("selected");
        selected.classList.remove("selected");
      }
    }

    if (!newJDK) return;

    //invoke the setJDK command
    loading = true;
    const jdk = await invoke("set_jdk", { jdk: newJDK.textContent });
    loading = false;
    loader.clear();

    //set selectedJDK to the new JDK
    selectedJDK = await invoke("get_current_jdk");
  });

  // window.isElevated = true;
</script>

<body>
  {#if !window.isElevated}
    <NotElevated />
  {:else}
    {#if loading}
      <Loader title="Loading..." bind:this={loader} />
    {/if}
    <fieldset>
      <legend>jman v{pack.version}</legend>

      <!-- <p>Currently running: <b>{selectedJDK}</b></p> -->
      {#if !selectedJDK}
        <p class="red">
          No JDK is currently selected. Please select one from the list below.
        </p>
      {/if}
      <p>Installed JDKs detected:</p>
      {#if installedJDKS}
        <ul>
          {#each installedJDKS as jdk}
            {#if selectedJDK.includes(getVersionFromFolderName(jdk))}
              <li class="selected">{jdk}</li>
            {:else}
              <li>{jdk}</li>
            {/if}
          {/each}
        </ul>
      {/if}
    </fieldset>
  {/if}
</body>

<style>
  @import url("https://fonts.googleapis.com/css2?family=Rubik&display=swap");
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  * {
    font-family: Rubik, sans-serif;
    text-align: center;
    color: white;
  }

  .selected {
    list-style-type: ">   ";
    color: lime;
    font-weight: bold;
  }

  .red {
    color: red;
  }

  body {
    height: 92.5vh;
    margin: 0;
  }

  fieldset {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  legend {
    font-size: 2rem;
  }

  ul {
    padding: 0;
  }

  li {
    text-align: left;
  }
</style>
