<script lang="ts">
  import { permissionLabel } from "./permissions";
  import type { InstalledPlugin } from "./types";

  let {
    installed,
    onInstallFile,
    onInstallUrl,
    onUninstall,
  }: {
    installed: InstalledPlugin[];
    onInstallFile: () => void;
    onInstallUrl: (url: string) => void;
    onUninstall: (id: string) => void;
  } = $props();

  let url = $state("");
</script>

<div class="manager">
  <div class="bar">
    <button onclick={onInstallFile}>从文件安装</button>
    <input
      placeholder="粘贴 .pcp 链接后回车"
      bind:value={url}
      onkeydown={(e) => {
        if (e.key === "Enter" && url.trim()) {
          onInstallUrl(url.trim());
          url = "";
        }
      }}
    />
  </div>

  {#if installed.length === 0}
    <div class="empty">还没有安装任何插件。拖入 .pcp 文件，或从上面安装。</div>
  {/if}

  <ul class="list">
    {#each installed as p (p.id)}
      <li>
        <div class="row">
          <span class="name">{p.id}</span>
          <span class="ver">v{p.version} · {p.source}</span>
          <button class="rm" onclick={() => onUninstall(p.id)}>卸载</button>
        </div>
        <div class="perms">
          {p.granted.map(permissionLabel).join(" · ") || "无授权能力"}
        </div>
      </li>
    {/each}
  </ul>
</div>

<style>
  .manager {
    padding: 10px 12px;
    color: #e8e8ea;
  }
  .bar {
    display: flex;
    gap: 8px;
    margin-bottom: 10px;
  }
  .bar button {
    border: 0;
    background: var(--sel);
    color: #fff;
    border-radius: 6px;
    padding: 6px 12px;
    cursor: pointer;
    font-size: 13px;
    white-space: nowrap;
  }
  .bar input {
    flex: 1;
    border: 0;
    outline: 0;
    background: rgba(255, 255, 255, 0.06);
    color: #fff;
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 13px;
  }
  .empty {
    color: var(--muted);
    font-size: 13px;
    padding: 12px 4px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .list li {
    padding: 8px 4px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .name {
    font-size: 14px;
  }
  .ver {
    color: var(--muted);
    font-size: 12px;
  }
  .rm {
    margin-left: auto;
    border: 0;
    background: #5a2b2b;
    color: #ffd9d9;
    border-radius: 6px;
    padding: 4px 10px;
    cursor: pointer;
    font-size: 12px;
  }
  .perms {
    color: var(--muted);
    font-size: 12px;
    margin-top: 3px;
  }
</style>
