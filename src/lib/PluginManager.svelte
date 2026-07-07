<script lang="ts">
  import { permissionLabel } from "./permissions";
  import { isOfficialRegistry } from "./registry";
  import type { InstalledPlugin, RegistryPlugin, Plugin } from "./types";

  let {
    installed,
    registries,
    registryPlugins,
    officialRegistryUrl,
    onInstallFile,
    onInstallUrl,
    onUninstall,
    onAddRegistry,
    onRemoveRegistry,
    onRefreshRegistries,
    onInstallRegistryPlugin,
    plugins,
    autoOpen,
    onToggleAutoOpen,
    loading,
  }: {
    installed: InstalledPlugin[];
    registries: string[];
    registryPlugins: RegistryPlugin[];
    officialRegistryUrl: string;
    onInstallFile: () => void;
    onInstallUrl: (url: string) => void;
    onUninstall: (id: string) => void;
    onAddRegistry: (url: string) => void;
    onRemoveRegistry: (url: string) => void;
    onRefreshRegistries: () => void;
    onInstallRegistryPlugin: (plugin: RegistryPlugin) => void;
    plugins: Plugin[];
    autoOpen: Record<string, boolean>;
    onToggleAutoOpen: (id: string, value: boolean) => void;
    loading: boolean;
  } = $props();

  let url = $state("");
  let registryUrl = $state("");

  const nameOf = (id: string) => plugins.find((x) => x.id === id)?.name ?? id;

  function regStatus(p: RegistryPlugin): "none" | "installed" | "update" {
    const cur = installed.find((i) => i.id === p.id);
    if (!cur) return "none";
    return cur.version === p.version ? "installed" : "update";
  }
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

  <div class="registry">
    <div class="bar">
      <input
        placeholder="粘贴 registry.json 链接后回车"
        bind:value={registryUrl}
        onkeydown={(e) => {
          if (e.key === "Enter" && registryUrl.trim()) {
            onAddRegistry(registryUrl.trim());
            registryUrl = "";
          }
        }}
      />
      <button onclick={onRefreshRegistries} disabled={loading}>{loading ? "刷新中…" : "刷新注册中心"}</button>
    </div>

    {#each registries as r (r)}
      <div class="row registry-row">
        <span class="sub">{r}</span>
        {#if isOfficialRegistry(r, officialRegistryUrl)}
          <span class="tag">官方</span>
        {:else}
          <button class="rm" onclick={() => onRemoveRegistry(r)}>删除</button>
        {/if}
      </div>
    {/each}

    {#if registryPlugins.length > 0}
      <ul class="list registry-list">
        {#each registryPlugins as p (p.id)}
          <li>
            <div class="row">
              <span class="name">{p.name}</span>
              <span class="ver">v{p.version}</span>
              {#if regStatus(p) === "installed"}
                <button disabled>已安装</button>
              {:else if regStatus(p) === "update"}
                <button onclick={() => onInstallRegistryPlugin(p)}>更新到 v{p.version}</button>
              {:else}
                <button onclick={() => onInstallRegistryPlugin(p)}>安装</button>
              {/if}
            </div>
            <div class="perms">{p.description}</div>
            <div class="perms">{p.permissions.map(permissionLabel).join(" · ") || "无授权能力"}</div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if installed.length === 0}
    <div class="empty">还没有安装任何插件。拖入 .pcp 文件，或从上面安装。</div>
  {/if}

  <ul class="list">
    {#each installed as p (p.id)}
      <li>
        <div class="row">
          <span class="name">{nameOf(p.id)}</span>
          <span class="ver">v{p.version} · {p.source} · {p.id}</span>
          <button class="rm" onclick={() => onUninstall(p.id)}>卸载</button>
        </div>
        <div class="perms">
          {p.granted.map(permissionLabel).join(" · ") || "无授权能力"}
        </div>
        {#if plugins.find((x) => x.id === p.id)?.features.some((f) => f.triggers.some((t) => t.kind === "content"))}
          <label class="auto-open">
            <input
              type="checkbox"
              checked={autoOpen[p.id] !== false}
              onchange={(e) => onToggleAutoOpen(p.id, e.currentTarget.checked)}
            />
            剪贴板内容匹配时自动打开
          </label>
        {/if}
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
  .registry {
    margin-bottom: 10px;
  }
  .registry-row {
    padding: 4px;
  }
  .registry-list {
    margin-top: 6px;
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
  .tag {
    margin-left: auto;
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
  .auto-open {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 12px;
    margin-top: 4px;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
