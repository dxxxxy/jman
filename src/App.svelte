<script>
  import Greet from "./lib/Greet.svelte";
  import Loader from "./lib/Loader.svelte";
  import pack from "../package.json";
  import { invoke } from "@tauri-apps/api/tauri";
  import { get } from "svelte/store";

  let installedJDKS;
  let loading = false;
  let selectedJDK = "";

  (async () => {
    installedJDKS = Array.from(await invoke("get_installed_jdks"));
    console.log(installedJDKS);

    selectedJDK = await invoke("get_current_jdk");
    console.log(selectedJDK);

    console.log(selectedJDK.includes("jdk1.8.0_341"));

    getVersionFromFolderName("jdk1.8.0_341");
  })();

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

    setTimeout(() => {
      loading = false;
    }, 5000);

    console.log(jdk);
    selectedJDK = await invoke("get_current_jdk");
    document.querySelector(".red").remove();
  });
</script>

<body>
  {#if loading}
    <Loader />
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
            <li class="selected"><b>{jdk}</b></li>
          {:else}
            <li>{jdk}</li>
          {/if}
        {/each}
      </ul>
    {/if}
  </fieldset>
</body>

<style>
  @import url("https://fonts.googleapis.com/css2?family=Rubik&display=swap");
  * {
    font-family: Rubik, sans-serif;
    text-align: center;
  }

  .selected {
    list-style-type: ">   ";
    color: lime;
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
