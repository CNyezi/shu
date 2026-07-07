<script lang="ts">
  import { permissionLabel } from "./permissions";
  import { isOfficialRegistry } from "./registry";
  import { matchScore } from "./match";
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
    iconMap,
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
    iconMap: Record<string, string>;
  } = $props();

  let tab: "installed" | "market" = $state("installed");
  let search = $state("");
  let url = $state("");
  let registryUrl = $state("");

  const nameOf = (id: string) => plugins.find((x) => x.id === id)?.name ?? id;

  // id -> 可升级到的注册中心条目（同 id、版本号不同）
  const updates = $derived(
    new Map(
      registryPlugins
        .filter((rp) => installed.some((i) => i.id === rp.id && i.version !== rp.version))
        .map((rp) => [rp.id, rp] as const),
    ),
  );

  // 复用启动器的 matchScore（名称支持首字母），描述/id 用子串兜底
  function hit(q: string, name: string, extra: string): boolean {
    if (!q) return true;
    return matchScore(q, { name }) > 0 || extra.toLowerCase().includes(q.toLowerCase());
  }

  const shownInstalled = $derived(installed.filter((p) => hit(search.trim(), nameOf(p.id), p.id)));
  const shownMarket = $derived(
    registryPlugins.filter((p) => hit(search.trim(), p.name, `${p.description} ${p.id}`)),
  );

  const hasContentTrigger = (id: string) =>
    plugins.find((x) => x.id === id)?.features.some((f) => f.triggers.some((t) => t.kind === "content")) ?? false;

  function regStatus(p: RegistryPlugin): "none" | "installed" | "update" {
    const cur = installed.find((i) => i.id === p.id);
    if (!cur) return "none";
    return cur.version === p.version ? "installed" : "update";
  }
</script>

<div class="manager">
  <div class="top">
    <div class="tabs" role="tablist">
      <button
        class="tab"
        class:active={tab === "installed"}
        role="tab"
        aria-selected={tab === "installed"}
        onclick={() => (tab = "installed")}
      >
        已安装 <span class="count">{installed.length}</span>
        {#if updates.size > 0}<span class="badge">{updates.size}</span>{/if}
      </button>
      <button
        class="tab"
        class:active={tab === "market"}
        role="tab"
        aria-selected={tab === "market"}
        onclick={() => (tab = "market")}
      >
        插件市场
      </button>
    </div>
    <input class="search" placeholder="搜索插件…" bind:value={search} />
  </div>

  {#if tab === "installed"}
    {#if installed.length === 0}
      <div class="empty">还没有安装任何插件——去「插件市场」看看，或拖入 .pcp 文件。</div>
    {:else if shownInstalled.length === 0}
      <div class="empty">无匹配插件</div>
    {:else}
      <ul class="list">
        {#each shownInstalled as p (p.id)}
          {@const up = updates.get(p.id)}
          <li class="item">
            {#if iconMap[p.id]}
              <img class="picon" src={iconMap[p.id]} alt="" />
            {:else}
              <span class="picon fallback">{nameOf(p.id).slice(0, 1)}</span>
            {/if}
            <div class="meta">
              <div class="head">
                <span class="name">{nameOf(p.id)}</span>
                <span class="ver">v{p.version}</span>
                {#if up}<span class="up-pill">↑ v{up.version}</span>{/if}
              </div>
              <div class="sub">{p.granted.map(permissionLabel).join(" · ") || "无授权能力"} · {p.id}</div>
              {#if hasContentTrigger(p.id)}
                <label class="auto-open">
                  <input
                    type="checkbox"
                    checked={autoOpen[p.id] !== false}
                    onchange={(e) => onToggleAutoOpen(p.id, e.currentTarget.checked)}
                  />
                  剪贴板内容匹配时自动打开
                </label>
              {/if}
            </div>
            <div class="actions">
              {#if up}
                <button class="primary" onclick={() => onInstallRegistryPlugin(up)}>更新</button>
              {/if}
              <button class="danger" onclick={() => onUninstall(p.id)}>卸载</button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  {:else}
    <div class="market-head">
      <span class="hint">{registries.length} 个源 · {registryPlugins.length} 个插件</span>
      <button class="ghost" onclick={onRefreshRegistries} disabled={loading}>
        {loading ? "刷新中…" : "刷新"}
      </button>
    </div>

    {#if registryPlugins.length === 0}
      <div class="empty">{loading ? "正在加载注册中心…" : "注册中心暂无插件，试试「刷新」。"}</div>
    {:else if shownMarket.length === 0}
      <div class="empty">无匹配插件</div>
    {:else}
      <ul class="list">
        {#each shownMarket as p (p.id)}
          <li class="item">
            <span class="picon fallback">{p.name.slice(0, 1)}</span>
            <div class="meta">
              <div class="head">
                <span class="name">{p.name}</span>
                <span class="ver">v{p.version}</span>
              </div>
              <div class="desc">{p.description}</div>
              <div class="sub">{p.permissions.map(permissionLabel).join(" · ") || "无授权能力"}</div>
            </div>
            <div class="actions">
              {#if regStatus(p) === "installed"}
                <button class="primary" disabled>已安装</button>
              {:else if regStatus(p) === "update"}
                <button class="primary" onclick={() => onInstallRegistryPlugin(p)}>更新到 v{p.version}</button>
              {:else}
                <button class="primary" onclick={() => onInstallRegistryPlugin(p)}>安装</button>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}

    <details class="advanced">
      <summary>安装来源与高级</summary>
      <div class="adv-row">
        <button class="ghost" onclick={onInstallFile}>从文件安装</button>
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
      <div class="adv-row">
        <input
          placeholder="粘贴 registry.json 链接后回车，添加新源"
          bind:value={registryUrl}
          onkeydown={(e) => {
            if (e.key === "Enter" && registryUrl.trim()) {
              onAddRegistry(registryUrl.trim());
              registryUrl = "";
            }
          }}
        />
      </div>
      {#each registries as r (r)}
        <div class="src-row">
          <span class="src">{r}</span>
          {#if isOfficialRegistry(r, officialRegistryUrl)}
            <span class="tag">官方</span>
          {:else}
            <button class="danger sm" onclick={() => onRemoveRegistry(r)}>删除</button>
          {/if}
        </div>
      {/each}
    </details>
  {/if}
</div>

<style>
  .manager {
    padding: 0 10px 10px;
    color: #e8e8ea;
    /* 插件多起来时窗口不能无限长高，管理器自身滚动 */
    max-height: 480px;
    overflow-y: auto;
  }

  /* ---- 顶部：tab + 搜索（滚动时吸顶） ---- */
  .top {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0 0;
    background: var(--bg);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    margin-bottom: 6px;
  }

  .tabs {
    display: flex;
    gap: 2px;
  }

  .tab {
    position: relative;
    border: 0;
    background: none;
    color: var(--muted);
    font-size: 13px;
    padding: 5px 10px 9px;
    cursor: pointer;
    transition: color 0.15s;
  }

  .tab:hover {
    color: #cfcfd2;
  }

  .tab.active {
    color: #fff;
  }

  .tab::after {
    content: "";
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: 0;
    height: 2px;
    border-radius: 1px;
    background: var(--sel);
    transform: scaleX(0);
    transition: transform 0.18s ease;
  }

  .tab.active::after {
    transform: scaleX(1);
  }

  .count {
    color: var(--muted);
    font-size: 11px;
  }

  .badge {
    display: inline-block;
    background: var(--sel);
    color: #fff;
    font-size: 10px;
    line-height: 1;
    padding: 2px 5px;
    border-radius: 8px;
    margin-left: 3px;
    vertical-align: 2px;
  }

  .search {
    margin-left: auto;
    width: 180px;
    border: 0;
    outline: 0;
    background: rgba(255, 255, 255, 0.06);
    color: #fff;
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 12px;
    margin-bottom: 7px;
    transition: background 0.15s;
  }

  .search:focus {
    background: rgba(255, 255, 255, 0.1);
  }

  .search::placeholder {
    color: var(--muted);
  }

  /* ---- 列表 ---- */
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .item {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 9px 8px;
    border-radius: 8px;
    transition: background 0.12s;
  }

  .item:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .item + .item {
    margin-top: 2px;
  }

  .picon {
    width: 28px;
    height: 28px;
    border-radius: 7px;
    flex: 0 0 auto;
    margin-top: 1px;
  }

  .picon.fallback {
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, rgba(47, 111, 237, 0.28), rgba(47, 111, 237, 0.1));
    color: #9db8f5;
    font-size: 13px;
    font-weight: 600;
  }

  .meta {
    flex: 1;
    min-width: 0;
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .name {
    font-size: 13.5px;
    font-weight: 500;
  }

  .ver {
    color: var(--muted);
    font-size: 11px;
  }

  .up-pill {
    background: rgba(47, 111, 237, 0.2);
    color: #9db8f5;
    font-size: 10.5px;
    padding: 1.5px 7px;
    border-radius: 9px;
    white-space: nowrap;
  }

  .desc {
    font-size: 12px;
    color: #b9b9be;
    margin-top: 2px;
  }

  .sub {
    font-size: 11px;
    color: var(--muted);
    margin-top: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .auto-open {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 12px;
    margin-top: 5px;
    cursor: pointer;
    width: fit-content;
  }

  .auto-open input {
    accent-color: var(--sel);
  }

  .actions {
    display: flex;
    gap: 6px;
    flex: 0 0 auto;
    align-self: center;
  }

  /* ---- 按钮层级 ---- */
  .primary {
    border: 0;
    background: var(--sel);
    color: #fff;
    border-radius: 6px;
    padding: 5px 12px;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
    transition: filter 0.12s;
  }

  .primary:hover {
    filter: brightness(1.12);
  }

  .primary:disabled {
    background: rgba(255, 255, 255, 0.08);
    color: var(--muted);
    cursor: default;
    filter: none;
  }

  .danger {
    border: 1px solid rgba(229, 112, 122, 0.35);
    background: none;
    color: #e5707a;
    border-radius: 6px;
    padding: 4px 11px;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.12s;
  }

  .danger:hover {
    background: rgba(229, 112, 122, 0.12);
  }

  .danger.sm {
    padding: 2px 8px;
    font-size: 11px;
  }

  .ghost {
    border: 1px solid rgba(255, 255, 255, 0.14);
    background: none;
    color: #cfcfd2;
    border-radius: 6px;
    padding: 4px 11px;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.12s;
  }

  .ghost:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .ghost:disabled {
    opacity: 0.45;
    cursor: default;
  }

  /* ---- 市场页眉 / 空态 ---- */
  .market-head {
    display: flex;
    align-items: center;
    padding: 2px 8px 6px;
  }

  .hint {
    color: var(--muted);
    font-size: 11.5px;
  }

  .market-head .ghost {
    margin-left: auto;
  }

  .empty {
    color: var(--muted);
    font-size: 12.5px;
    text-align: center;
    padding: 24px 0 16px;
  }

  /* ---- 折叠的低频区 ---- */
  .advanced {
    margin-top: 10px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    padding-top: 8px;
  }

  .advanced summary {
    cursor: pointer;
    color: var(--muted);
    font-size: 12px;
    padding: 4px 8px;
    border-radius: 6px;
    user-select: none;
    width: fit-content;
    transition: color 0.15s;
  }

  .advanced summary:hover {
    color: #cfcfd2;
  }

  .adv-row {
    display: flex;
    gap: 8px;
    margin: 8px 8px 0;
  }

  .adv-row input {
    flex: 1;
    border: 0;
    outline: 0;
    background: rgba(255, 255, 255, 0.06);
    color: #fff;
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 12px;
  }

  .adv-row input::placeholder {
    color: var(--muted);
  }

  .src-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 8px 0;
  }

  .src {
    flex: 1;
    color: var(--muted);
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag {
    color: var(--muted);
    font-size: 11px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    padding: 1px 7px;
    border-radius: 8px;
  }
</style>
