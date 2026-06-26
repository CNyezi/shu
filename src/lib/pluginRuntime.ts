import {
  capabilities,
  canUseCapability,
  effectivePermissions,
  storageGet,
  storageSet,
  storageRemove,
  storageKeys,
} from "./host";
import type { Plugin, Feature } from "./types";

/**
 * Bootstrap injected into every plugin's sandboxed iframe. It defines the
 * `window.host` API as thin postMessage wrappers — the plugin never touches
 * Tauri's `invoke` directly. All "plugin -> system" calls go through the host
 * shell, which enforces the permission whitelist (see mountPlugin below).
 */
const BOOTSTRAP = `
(function () {
  var _ctx = { input: "" };
  var _inputCb = null;
  var _pending = new Map();
  var _seq = 0;
  function call(name, args) {
    return new Promise(function (res, rej) {
      var id = ++_seq;
      _pending.set(id, { res: res, rej: rej });
      parent.postMessage({ __pc: true, kind: "capability", id: id, name: name, args: args || {} }, "*");
    });
  }
  function storageCall(op, args) {
    return new Promise(function (res, rej) {
      var id = ++_seq;
      _pending.set(id, { res: res, rej: rej });
      parent.postMessage({ __pc: true, kind: "storage", id: id, op: op, args: args || {} }, "*");
    });
  }
  window.addEventListener("message", function (e) {
    var m = e.data;
    if (!m || !m.__pc) return;
    if (m.kind === "capability-result") {
      var p = _pending.get(m.id);
      if (!p) return;
      _pending.delete(m.id);
      if (m.ok) p.res(m.value); else p.rej(new Error(m.error));
    } else if (m.kind === "input") {
      _ctx.input = m.input;
      if (_inputCb) _inputCb(m.input);
    } else if (m.kind === "init") {
      _ctx = m.context || _ctx;
    }
  });
  window.host = {
    onInput: function (cb) { _inputCb = cb; },
    getContext: function () { return _ctx; },
    setResults: function (results) { parent.postMessage({ __pc: true, kind: "setResults", results: results }, "*"); },
    redirect: function (code) { parent.postMessage({ __pc: true, kind: "redirect", code: code }, "*"); },
    close: function () { parent.postMessage({ __pc: true, kind: "close" }, "*"); },
    clipboard: {
      read: function () { return call("clipboard.read"); },
      write: function (text) { return call("clipboard.write", { text: text }); },
      readImage: function () { return call("clipboard.readImage"); },
      writeImage: function (dataUrl) { return call("clipboard.writeImage", { dataUrl: dataUrl }); },
      readFiles: function () { return call("clipboard.readFiles"); },
      writeFiles: function (paths) { return call("clipboard.writeFiles", { paths: paths }); }
    },
    openUrl: function (url) { return call("shell.openUrl", { url: url }); },
    openPath: function (path) { return call("shell.openPath", { path: path }); },
    hosts: {
      read: function () { return call("hosts.read"); },
      write: function (content) { return call("hosts.write", { content: content }); }
    },
    fs: {
      scopes: function () { return call("fs.scopes"); },
      readText: function (p) { return call("fs.readText", { path: p }); },
      readBytes: function (p) { return call("fs.readBytes", { path: p }); },
      list: function (p) { return call("fs.list", { path: p }); },
      exists: function (p) { return call("fs.exists", { path: p }); },
      stat: function (p) { return call("fs.stat", { path: p }); },
      writeText: function (p, c) { return call("fs.writeText", { path: p, content: c }); },
      writeBytes: function (p, b64) { return call("fs.writeBytes", { path: p, base64Data: b64 }); },
      mkdir: function (p) { return call("fs.mkdir", { path: p }); },
      remove: function (p) { return call("fs.remove", { path: p }); }
    },
    notify: function (title, body) { return call("notification", { title: title, body: body }); },
    http: function (url, opts) {
      opts = opts || {};
      return call("network.http", { url: url, method: opts.method, headers: opts.headers, body: opts.body });
    },
    storage: {
      get: function (key) { return storageCall("get", { key: key }); },
      set: function (key, value) { return storageCall("set", { key: key, value: value }); },
      remove: function (key) { return storageCall("remove", { key: key }); },
      keys: function () { return storageCall("keys", {}); }
    }
  };
})();
`;

function injectBootstrap(html: string): string {
  const tag = `<script>${BOOTSTRAP}</script>`;
  if (/<head[^>]*>/i.test(html)) {
    return html.replace(/<head[^>]*>/i, (m) => m + tag);
  }
  return tag + html;
}

function wrapLogic(js: string): string {
  return `<!doctype html><html><head><meta charset="utf-8"><script>${BOOTSTRAP}</script></head><body><script>${js}</script></body></html>`;
}

export type PluginController = {
  sendInput(value: string): void;
  destroy(): void;
};

export type PluginHooks = {
  onSetResults?(results: unknown[]): void;
  onRedirect?(code: string): void;
  onClose?(): void;
};

/**
 * Mount a plugin feature into `container` as a sandboxed iframe and wire up the
 * mediated capability bridge. UI features render visibly; logic features run
 * hidden and report back via `onSetResults`.
 */
export function mountPlugin(
  container: HTMLElement,
  plugin: Plugin,
  feature: Feature,
  code: string,
  hooks: PluginHooks,
): PluginController {
  const iframe = document.createElement("iframe");
  // allow-scripts ONLY -> opaque origin, no same-origin, no parent access, no Tauri.
  iframe.setAttribute("sandbox", "allow-scripts");
  iframe.style.cssText =
    feature.type === "ui"
      ? "width:100%;height:100%;border:0;background:#fff;"
      : "display:none;";
  iframe.srcdoc = feature.type === "ui" ? injectBootstrap(code) : wrapLogic(code);

  const whitelist = effectivePermissions(plugin.permissions, plugin.granted);

  function reply(id: number, ok: boolean, value?: unknown, error?: string) {
    iframe.contentWindow?.postMessage(
      { __pc: true, kind: "capability-result", id, ok, value, error },
      "*",
    );
  }

  async function handleCapability(m: any) {
    const args = m.args || {};
    if (m.name.startsWith("fs.")) {
      // fs is scope-enforced in Rust (the permission depends on which directory
      // the path falls in). Inject the granted permission set + plugin id here —
      // both come from trusted host state, never from the iframe message.
      args.granted = whitelist;
      args.pluginId = plugin.id;
    } else if (!canUseCapability(plugin.permissions, plugin.granted, m.name)) {
      reply(m.id, false, undefined, `permission denied: ${m.name}`);
      return;
    }
    const impl = capabilities[m.name];
    if (!impl) {
      reply(m.id, false, undefined, `unknown capability: ${m.name}`);
      return;
    }
    try {
      const value = await impl(args);
      reply(m.id, true, value);
    } catch (err) {
      reply(m.id, false, undefined, String(err));
    }
  }

  // Storage needs no permission — it's the plugin's own namespaced data. The
  // plugin id is injected here (from mountPlugin), never trusted from the iframe.
  async function handleStorage(m: any) {
    try {
      const pid = plugin.id;
      let value: unknown;
      if (m.op === "get") value = await storageGet(pid, m.args.key);
      else if (m.op === "set") value = await storageSet(pid, m.args.key, m.args.value);
      else if (m.op === "remove") value = await storageRemove(pid, m.args.key);
      else if (m.op === "keys") value = await storageKeys(pid);
      else {
        reply(m.id, false, undefined, "unknown storage op");
        return;
      }
      reply(m.id, true, value);
    } catch (err) {
      reply(m.id, false, undefined, String(err));
    }
  }

  function onMessage(e: MessageEvent) {
    if (e.source !== iframe.contentWindow) return;
    const m = e.data;
    if (!m || !m.__pc) return;
    if (m.kind === "capability") void handleCapability(m);
    else if (m.kind === "storage") void handleStorage(m);
    else if (m.kind === "setResults") hooks.onSetResults?.(m.results);
    else if (m.kind === "redirect") hooks.onRedirect?.(m.code);
    else if (m.kind === "close") hooks.onClose?.();
  }

  window.addEventListener("message", onMessage);
  iframe.addEventListener("load", () => {
    iframe.contentWindow?.postMessage(
      { __pc: true, kind: "init", context: { input: "" } },
      "*",
    );
  });

  container.appendChild(iframe);

  return {
    sendInput(value: string) {
      iframe.contentWindow?.postMessage(
        { __pc: true, kind: "input", input: value },
        "*",
      );
    },
    destroy() {
      window.removeEventListener("message", onMessage);
      iframe.remove();
    },
  };
}
