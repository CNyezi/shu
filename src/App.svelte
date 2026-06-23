<script lang="ts">
  import { onMount, tick } from "svelte";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
  import {
    listApps,
    launchApp,
    listPlugins,
    readPluginFile,
    readClipboard,
    appIcon,
    hideWindow,
    setAutoHide,
    inspectPackage,
    downloadPackage,
    installPackage,
    uninstallPlugin,
    listInstalled,
  } from "./lib/host";
  import { mountPlugin, type PluginController } from "./lib/pluginRuntime";
  import type { AppEntry, Plugin, Feature, ResultItem, InstalledPlugin, PackageInspect } from "./lib/types";
  import PluginManager from "./lib/PluginManager.svelte";
  import InstallConsent from "./lib/InstallConsent.svelte";

  let query = $state("");
  let apps: AppEntry[] = $state([]);
  let plugins: Plugin[] = $state([]);
  let results: ResultItem[] = $state([]);
  let selected = $state(0);

  // Clipboard-content recommendations, shown when the query is empty.
  let clipRecommendations: ResultItem[] = $state([]);
  // pluginId -> icon data URL
  let iconMap: Record<string, string> = $state({});
  // app path -> icon data URL (null = no icon, '' = loading)
  let appIconMap: Record<string, string | null> = $state({});

  let mode: "search" | "plugin" | "manager" | "consent" = $state("search");
  let installed: InstalledPlugin[] = $state([]);
  let consentInfo: PackageInspect | null = $state(null);
  let pendingPath: string | null = $state(null);
  let pendingOrigin: string | null = $state(null);
  let toast = $state("");
  let composing = $state(false); // IME composition in progress (e.g. pinyin)
  let activeLabel = $state("");
  let activeFeatureType: "ui" | "logic" = $state("ui");
  let pluginResults: any[] = $state([]);

  let controller: PluginController | null = null;
  let inputEl: HTMLInputElement | undefined = $state();
  let pluginHost: HTMLDivElement | undefined = $state();
  let rootEl: HTMLDivElement | undefined = $state();

  const WIN_W = 680;

  // Resize the window to fit the rendered card, so an empty launcher is just
  // the search box and it grows as results / a plugin appear.
  let curH = 60;
  let firstResize = true;
  let animTimer: ReturnType<typeof setInterval> | null = null;

  function applyHeight(h: number) {
    try {
      void getCurrentWindow().setSize(new LogicalSize(WIN_W, h));
    } catch {
      /* ignore */
    }
  }

  // Animate the native window height (easeOutCubic) so the panel expands /
  // collapses smoothly. Driven by setInterval (rAF stalls during the native
  // window relayout) and guaranteed to land exactly on the target.
  function animateHeight(target: number) {
    if (animTimer) clearInterval(animTimer);
    if (firstResize) {
      firstResize = false;
      curH = target;
      applyHeight(target);
      return;
    }
    if (curH === target) return;
    const start = curH;
    const t0 = performance.now();
    const dur = 140;
    animTimer = setInterval(() => {
      const k = Math.min(1, (performance.now() - t0) / dur);
      const ease = 1 - Math.pow(1 - k, 3);
      curH = Math.round(start + (target - start) * ease);
      applyHeight(curH);
      if (k >= 1) {
        curH = target;
        applyHeight(target);
        if (animTimer) clearInterval(animTimer);
        animTimer = null;
      }
    }, 16);
  }

  async function resizeToContent() {
    await tick();
    if (!rootEl) return;
    animateHeight(Math.ceil(rootEl.getBoundingClientRect().height));
  }

  // Re-fit whenever layout-affecting state changes.
  $effect(() => {
    void results.length;
    void mode;
    void activeFeatureType;
    void pluginResults.length;
    void resizeToContent();
  });

  onMount(async () => {
    apps = await listApps();
    plugins = await listPlugins();
    await loadIcons();
    inputEl?.focus();
    await refreshClipboard();

    // On every re-show via the hotkey: re-read clipboard and re-recommend.
    await getCurrentWindow().listen("pc:shown", async () => {
      if (mode === "plugin") exitPlugin();
      await tick();
      inputEl?.focus();
      void listApps().then((a) => (apps = a)); // refresh app list in background
      await refreshClipboard();
    });

    await getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "drop") {
        const file = event.payload.paths.find((p) => p.endsWith(".pcp"));
        if (file) void beginInstallFromPath(file, file);
      }
    });
  });

  async function loadIcons() {
    const map: Record<string, string> = {};
    for (const p of plugins) {
      if (!p.icon) continue;
      try {
        const svg = await readPluginFile(p._dir, p.icon);
        map[p.id] = "data:image/svg+xml," + encodeURIComponent(svg);
      } catch {
        /* missing icon is fine */
      }
    }
    iconMap = map;
  }

  async function loadAppIcons(items: ResultItem[]) {
    // Only the first dozen are visible; avoid flooding the icon extractor.
    for (const item of items.slice(0, 12)) {
      if (item.kind !== "app" || item.path in appIconMap) continue;
      appIconMap[item.path] = ""; // mark loading to avoid refetch
      try {
        appIconMap[item.path] = (await appIcon(item.path)) ?? null;
      } catch {
        appIconMap[item.path] = null;
      }
    }
  }

  function iconFor(item: ResultItem): string | null {
    if (item.kind === "feature") return iconMap[item.plugin.id] ?? null;
    return appIconMap[item.path] || null;
  }

  // --- content detection (host side, extensible) ---
  function detectContentKind(text: string): string | null {
    const t = text.trim();
    if (!t) return null;
    if (
      (t.startsWith("{") && t.endsWith("}")) ||
      (t.startsWith("[") && t.endsWith("]"))
    ) {
      try {
        JSON.parse(t);
        return "json";
      } catch {
        /* not json */
      }
    }
    return null;
  }

  function featuresForContent(kind: string): ResultItem[] {
    const items: ResultItem[] = [];
    for (const p of plugins) {
      for (const f of p.features) {
        if (f.triggers.some((t) => t.kind === "content" && t.value === kind)) {
          items.push({
            kind: "feature",
            title: p.name,
            subtitle: `处理 ${kind} 内容`,
            plugin: p,
            feature: f,
          });
        }
      }
    }
    return items;
  }

  async function refreshClipboard() {
    let clip: { kind: string; text: string };
    try {
      clip = await readClipboard();
    } catch {
      clip = { kind: "empty", text: "" };
    }
    const kind = clip.kind === "text" ? detectContentKind(clip.text) : null;
    const matches = kind ? featuresForContent(kind) : [];

    // Exactly one handler -> open it directly. Otherwise recommend.
    if (matches.length === 1) {
      clipRecommendations = [];
      const m = matches[0];
      if (m.kind === "feature") void enterFeature(m.plugin, m.feature);
      return;
    }
    clipRecommendations = matches;
    if (mode === "search" && query.trim() === "") results = clipRecommendations;
  }

  function computeResults(q: string) {
    if (!q) {
      results = clipRecommendations;
      selected = 0;
      return;
    }
    const items: ResultItem[] = [];
    const ql = q.toLowerCase();
    for (const a of apps) {
      if (a.name.toLowerCase().includes(ql)) {
        items.push({ kind: "app", title: a.name, subtitle: a.path, path: a.path });
      }
    }
    // Plugins behave like apps: a feature matched by keyword-prefix, plugin
    // name, or a regex trigger shows up as a selectable result (Enter to open).
    for (const p of plugins) {
      for (const f of p.features) {
        let keyword: string | undefined;
        let matched = p.name.toLowerCase().includes(ql);
        for (const t of f.triggers) {
          if (t.kind === "keyword" && t.value.toLowerCase().startsWith(ql)) {
            matched = true;
            keyword = t.value;
          } else if (t.kind === "regex") {
            try {
              if (new RegExp(t.value).test(q)) matched = true;
            } catch {
              /* ignore bad regex */
            }
          }
        }
        if (matched) {
          items.push({
            kind: "feature",
            title: p.name,
            subtitle: keyword ? `插件 · ${keyword}` : "插件",
            plugin: p,
            feature: f,
          });
        }
      }
    }
    items.sort((a, b) => {
      const ap = a.title.toLowerCase().startsWith(ql) ? 0 : 1;
      const bp = b.title.toLowerCase().startsWith(ql) ? 0 : 1;
      return ap - bp || a.title.localeCompare(b.title);
    });
    results = items.slice(0, 50);
    selected = 0;
    void loadAppIcons(results);
  }

  function handleInput() {
    // Ignore intermediate IME composition events (pinyin); handled on commit.
    if (composing) return;
    if (mode === "manager" || mode === "consent") return; // these views own their own inputs
    if (mode === "plugin") {
      controller?.sendInput(query);
      return;
    }
    const q = query.trim();
    const token = q.split(/\s+/)[0] ?? "";
    if (token === "插件" || token === "plugins") {
      void openManager();
      return;
    }
    // Plugins are no longer auto-entered on keyword match — they appear as
    // results and are opened on Enter / click (like apps).
    computeResults(q);
  }

  async function refreshInstalled() {
    installed = await listInstalled();
  }

  function showToast(msg: string) {
    toast = msg;
    setTimeout(() => (toast = ""), 2000);
  }

  async function beginInstallFromPath(path: string, origin: string) {
    void setAutoHide(false); // consent view must not vanish on blur
    try {
      consentInfo = await inspectPackage(path);
      pendingPath = path;
      pendingOrigin = origin;
      mode = "consent";
    } catch (e) {
      showToast("无法读取插件包：" + String(e));
    }
  }

  async function installFromFile() {
    const picked = await openFileDialog({
      multiple: false,
      filters: [{ name: "pc-tool 插件", extensions: ["pcp"] }],
    });
    if (typeof picked === "string") await beginInstallFromPath(picked, picked);
  }

  async function installFromUrl(url: string) {
    try {
      const path = await downloadPackage(url);
      await beginInstallFromPath(path, url); // origin = the URL, not the temp file
    } catch (e) {
      showToast("下载失败：" + String(e));
    }
  }

  async function approveInstall() {
    if (!consentInfo || !pendingPath || !pendingOrigin) return;
    try {
      await installPackage(pendingPath, consentInfo.manifest.permissions, pendingOrigin);
      plugins = await listPlugins();
      await refreshInstalled();
      showToast(`已安装 ${consentInfo.manifest.name}`);
    } catch (e) {
      showToast("安装失败：" + String(e));
    }
    consentInfo = null;
    pendingPath = null;
    pendingOrigin = null;
    mode = "manager";
  }

  function cancelInstall() {
    consentInfo = null;
    pendingPath = null;
    pendingOrigin = null;
    mode = "manager";
  }

  async function doUninstall(id: string) {
    await uninstallPlugin(id);
    plugins = await listPlugins();
    await refreshInstalled();
    showToast("已卸载");
  }

  async function openManager() {
    void setAutoHide(false); // keep window alive for file dialog / drag-drop
    query = "";
    results = [];
    await refreshInstalled();
    mode = "manager";
  }

  function exitManager() {
    void setAutoHide(true);
    mode = "search";
    query = "";
    computeResults("");
  }

  async function enterFeature(plugin: Plugin, feature: Feature) {
    controller?.destroy();
    controller = null;
    query = "";
    results = [];
    pluginResults = [];
    mode = "plugin";
    activeLabel = plugin.name;
    activeFeatureType = feature.type;

    let code: string;
    try {
      code = await readPluginFile(plugin._dir, feature.entry);
    } catch (e) {
      activeLabel = `加载失败: ${String(e)}`;
      return;
    }

    await tick();
    if (!pluginHost) return;
    controller = mountPlugin(pluginHost, plugin, feature, code, {
      onSetResults: (r) => (pluginResults = r),
      onRedirect: (codeName) => {
        const f = plugin.features.find((x) => x.code === codeName);
        if (f) void enterFeature(plugin, f);
      },
      onClose: () => exitPlugin(),
    });
    inputEl?.focus();
  }

  function exitPlugin() {
    controller?.destroy();
    controller = null;
    mode = "search";
    activeLabel = "";
    query = "";
    pluginResults = [];
    computeResults("");
    void tick().then(() => inputEl?.focus());
  }

  function activate(item: ResultItem | undefined) {
    if (!item) return;
    if (item.kind === "app") {
      void launchApp(item.path);
      void hideWindow();
    } else {
      void enterFeature(item.plugin, item.feature);
    }
  }

  function goBack() {
    if (mode === "consent") cancelInstall();
    else if (mode === "manager") exitManager();
    else if (mode === "plugin") exitPlugin();
    else void hideWindow();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.isComposing || composing) return; // let the IME handle composition keys
    if (e.key === "Escape") {
      e.preventDefault();
      goBack();
      return;
    }
    if (mode === "plugin") return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selected = Math.min(selected + 1, results.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selected = Math.max(selected - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      activate(results[selected]);
    }
  }
</script>

<div class="root" bind:this={rootEl}>
  <div class="bar">
    {#if mode !== "search"}
      <button class="back" onclick={goBack} title="返回 (Esc)">←</button>
      <span class="label">{mode === "manager" ? "插件管理" : mode === "consent" ? "安装插件" : activeLabel}</span>
    {/if}
    <input
      bind:this={inputEl}
      bind:value={query}
      oninput={handleInput}
      oncompositionstart={() => (composing = true)}
      oncompositionend={() => {
        composing = false;
        handleInput();
      }}
      onkeydown={onKeydown}
      placeholder={mode === "plugin" ? "输入以传给插件…" : "搜索应用，或输入关键词（如 json）"}
      autocomplete="off"
      spellcheck="false"
    />
  </div>

  {#if mode === "consent" && consentInfo}
    <InstallConsent
      info={consentInfo}
      onApprove={approveInstall}
      onCancel={cancelInstall}
    />
  {:else if mode === "manager"}
    <PluginManager
      {installed}
      onInstallFile={installFromFile}
      onInstallUrl={installFromUrl}
      onUninstall={doUninstall}
    />
  {:else if mode === "plugin"}
    <div class="content" class:hidden={activeFeatureType === "logic"}>
      <div class="plugin-host" bind:this={pluginHost}></div>
    </div>
    {#if activeFeatureType === "logic" && pluginResults.length > 0}
      <ul class="results">
        {#each pluginResults as r, i (i)}
          <li><span class="title">{r.title ?? r}</span><span class="sub">{r.subtitle ?? ""}</span></li>
        {/each}
      </ul>
    {/if}
  {:else if results.length > 0}
    <ul class="results">
      {#each results as item, i (item.kind + item.title + i)}
        <li
          class:sel={i === selected}
          onmousedown={() => activate(item)}
          role="option"
          aria-selected={i === selected}
          tabindex="-1"
        >
          {#if iconFor(item)}
            <img class="icon" src={iconFor(item)} alt="" />
          {:else}
            <span class="icon placeholder"></span>
          {/if}
          <span class="meta">
            <span class="title">{item.title}</span>
            <span class="sub">{item.subtitle}</span>
          </span>
        </li>
      {/each}
    </ul>
  {/if}

  {#if toast}
    <div class="toast">{toast}</div>
  {/if}
</div>

<style>
  .root {
    display: flex;
    flex-direction: column;
    height: auto;
    background: var(--bg);
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: var(--bar);
  }

  .back {
    border: 0;
    background: #3a3a3e;
    color: #e8e8ea;
    width: 26px;
    height: 26px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
  }

  .label {
    color: var(--muted);
    font-size: 13px;
    white-space: nowrap;
  }

  input {
    flex: 1;
    border: 0;
    outline: 0;
    background: transparent;
    color: #fff;
    font-size: 18px;
  }

  input::placeholder {
    color: var(--muted);
  }

  .content {
    height: 440px;
  }

  .content.hidden {
    height: 0;
  }

  .plugin-host {
    width: 100%;
    height: 100%;
  }

  .results {
    list-style: none;
    margin: 0;
    padding: 6px;
    overflow-y: auto;
    max-height: 380px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  .results li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border-radius: 7px;
    cursor: pointer;
  }

  .results li.sel {
    background: var(--sel);
  }

  .icon {
    width: 22px;
    height: 22px;
    border-radius: 5px;
    flex: 0 0 auto;
  }

  .icon.placeholder {
    background: rgba(255, 255, 255, 0.06);
  }

  .meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .title {
    font-size: 14px;
  }

  .sub {
    font-size: 11px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .results li.sel .sub {
    color: #d3e0ff;
  }

  .toast {
    position: absolute;
    bottom: 10px;
    left: 50%;
    transform: translateX(-50%);
    background: #2f2f33;
    color: #fff;
    padding: 6px 14px;
    border-radius: 8px;
    font-size: 12px;
    white-space: nowrap;
  }
</style>
