<script lang="ts">
  import { onMount, tick } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    listApps,
    launchApp,
    listPlugins,
    readPluginFile,
    hideWindow,
  } from "./lib/host";
  import { mountPlugin, type PluginController } from "./lib/pluginRuntime";
  import type { AppEntry, Plugin, Feature, ResultItem } from "./lib/types";

  let query = $state("");
  let apps: AppEntry[] = $state([]);
  let plugins: Plugin[] = $state([]);
  let results: ResultItem[] = $state([]);
  let selected = $state(0);

  let mode: "search" | "plugin" = $state("search");
  let activeLabel = $state("");
  let activeFeatureType: "ui" | "logic" = $state("ui");
  let pluginResults: any[] = $state([]);

  let controller: PluginController | null = null;
  let inputEl: HTMLInputElement | undefined = $state();
  let pluginHost: HTMLDivElement | undefined = $state();

  onMount(async () => {
    apps = await listApps();
    plugins = await listPlugins();
    inputEl?.focus();
    computeResults("");

    // Refocus the search box whenever the window is re-shown via the hotkey.
    await getCurrentWindow().listen("pc:shown", async () => {
      if (mode === "plugin") exitPlugin();
      await tick();
      inputEl?.focus();
    });
  });

  function findKeywordFeature(
    token: string,
  ): { plugin: Plugin; feature: Feature } | null {
    for (const p of plugins) {
      for (const f of p.features) {
        for (const t of f.triggers) {
          if (t.kind === "keyword" && t.value === token) {
            return { plugin: p, feature: f };
          }
        }
      }
    }
    return null;
  }

  function computeResults(q: string) {
    const items: ResultItem[] = [];
    const ql = q.toLowerCase();
    if (ql) {
      for (const a of apps) {
        if (a.name.toLowerCase().includes(ql)) {
          items.push({ kind: "app", title: a.name, subtitle: a.path, path: a.path });
        }
      }
      for (const p of plugins) {
        for (const f of p.features) {
          for (const t of f.triggers) {
            if (t.kind !== "regex") continue;
            try {
              if (new RegExp(t.value).test(q)) {
                items.push({
                  kind: "feature",
                  title: p.name,
                  subtitle: `${f.code} 插件`,
                  plugin: p,
                  feature: f,
                });
              }
            } catch {
              /* ignore bad regex */
            }
          }
        }
      }
      // rank: prefix matches first
      items.sort((a, b) => {
        const ap = a.title.toLowerCase().startsWith(ql) ? 0 : 1;
        const bp = b.title.toLowerCase().startsWith(ql) ? 0 : 1;
        return ap - bp || a.title.localeCompare(b.title);
      });
    }
    results = items.slice(0, 50);
    selected = 0;
  }

  function handleInput() {
    if (mode === "plugin") {
      controller?.sendInput(query);
      return;
    }
    const q = query.trim();
    const token = q.split(/\s+/)[0] ?? "";
    const kw = token ? findKeywordFeature(token) : null;
    if (kw) {
      void enterFeature(kw.plugin, kw.feature);
      return;
    }
    computeResults(q);
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

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      if (mode === "plugin") exitPlugin();
      else void hideWindow();
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

<div class="root">
  <div class="bar">
    {#if mode === "plugin"}
      <button class="back" onclick={exitPlugin} title="返回 (Esc)">←</button>
      <span class="label">{activeLabel}</span>
    {/if}
    <input
      bind:this={inputEl}
      bind:value={query}
      oninput={handleInput}
      onkeydown={onKeydown}
      placeholder={mode === "plugin" ? "输入以传给插件…" : "搜索应用，或输入关键词（如 json）"}
      autocomplete="off"
      spellcheck="false"
    />
  </div>

  {#if mode === "plugin"}
    <div class="content" class:hidden={activeFeatureType === "logic"}>
      <div class="plugin-host" bind:this={pluginHost}></div>
    </div>
    {#if activeFeatureType === "logic"}
      <ul class="results">
        {#each pluginResults as r, i (i)}
          <li><span class="title">{r.title ?? r}</span><span class="sub">{r.subtitle ?? ""}</span></li>
        {/each}
      </ul>
    {/if}
  {:else}
    <ul class="results">
      {#each results as item, i (item.kind + item.title + i)}
        <li
          class:sel={i === selected}
          onmousedown={() => activate(item)}
          role="option"
          aria-selected={i === selected}
          tabindex="-1"
        >
          <span class="title">{item.title}</span>
          <span class="sub">{item.subtitle}</span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .root {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg);
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.08);
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: var(--bar);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
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
    flex: 1;
    min-height: 0;
  }

  .content.hidden {
    height: 0;
    flex: 0;
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
    flex: 1;
  }

  .results li {
    display: flex;
    flex-direction: column;
    padding: 7px 10px;
    border-radius: 7px;
    cursor: pointer;
  }

  .results li.sel {
    background: var(--sel);
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
</style>
