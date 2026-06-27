<script lang="ts">
  import {
    inspectPackage,
    installPackage,
    listInstalled,
    uninstallPlugin,
  } from "./host";
  import type { InstalledPlugin, PackageInspect } from "./types";
  import { testPackagePath } from "./testMode";

  let path = $state(testPackagePath(window.location.search));
  let info: PackageInspect | null = $state(null);
  let installed: InstalledPlugin[] = $state([]);
  let status = $state("Ready");

  async function refresh() {
    installed = await listInstalled();
  }

  async function inspect() {
    info = await inspectPackage(path);
    status = `Inspected ${info.manifest.id}`;
    await refresh();
  }

  async function install() {
    if (!info) await inspect();
    if (!info) return;
    await installPackage(path, info.manifest.permissions, path);
    status = `Installed ${info.manifest.id}`;
    await refresh();
  }

  async function remove() {
    const id = info?.manifest.id || "com.shu.json-preview";
    await uninstallPlugin(id);
    status = `Uninstalled ${id}`;
    await refresh();
  }

  refresh();
</script>

<main>
  <h1>枢 test</h1>
  <label>
    Package
    <input bind:value={path} />
  </label>
  <div class="actions">
    <button onclick={inspect}>Inspect</button>
    <button onclick={install}>Install</button>
    <button onclick={remove}>Uninstall</button>
  </div>
  <p data-testid="status">{status}</p>

  {#if info}
    <section>
      <h2>{info.manifest.name}</h2>
      <p>{info.manifest.id} v{info.manifest.version}</p>
      <p>sha256 {info.sha256}</p>
      <p>permissions {info.manifest.permissions.join(", ") || "none"}</p>
    </section>
  {/if}

  <section>
    <h2>Installed</h2>
    <ul>
      {#each installed as p (p.id)}
        <li>{p.id} v{p.version} [{p.granted.join(", ")}]</li>
      {/each}
    </ul>
  </section>
</main>

<style>
  main {
    min-height: 520px;
    padding: 18px;
    color: #eee;
    background: #1f2024;
    font: 13px/1.45 system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
  }

  h1 {
    margin: 0 0 14px;
    font-size: 20px;
  }

  h2 {
    margin: 16px 0 6px;
    font-size: 14px;
  }

  label {
    display: grid;
    gap: 6px;
  }

  input {
    box-sizing: border-box;
    width: 100%;
    border: 1px solid #4a4c55;
    border-radius: 6px;
    padding: 8px;
    color: #fff;
    background: #2a2b31;
  }

  .actions {
    display: flex;
    gap: 8px;
    margin: 12px 0;
  }

  button {
    border: 0;
    border-radius: 6px;
    padding: 7px 12px;
    color: #fff;
    background: #2f6fed;
    cursor: pointer;
  }

  p {
    margin: 6px 0;
    color: #cfd2dc;
    word-break: break-all;
  }

  ul {
    margin: 6px 0 0;
    padding-left: 18px;
  }
</style>
