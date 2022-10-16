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

    getVersionFromFolderName("jdk1.8.0_341");
  })();

  const getVersionFromFolderName = (jdk) => {
    const results = /jdk(|-)([0-9._]+)/.exec(jdk);
    return results[2];
  };

  //on up/down arrow, select the next/previous JDK in the ul
  document.addEventListener("keyup", async (e) => {
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

    //invoke the setJDK command
    const jdk = await invoke("set_jdk", { jdk: newJDK.textContent });
    console.log(jdk);
  });
</script>

<body>
  <fieldset>
    <legend align="left">jman v{pack.version}</legend>

    <!-- <p>Currently running: <b>{selectedJDK}</b></p> -->
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
  }

  .selected {
    list-style-type: ">   ";
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
