import { capabilities } from "./host";
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
      write: function (text) { return call("clipboard.write", { text: text }); }
    },
    openUrl: function (url) { return call("shell.openUrl", { url: url }); },
    openPath: function (path) { return call("shell.openPath", { path: path }); }
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

  const whitelist = new Set(plugin.permissions || []);

  function reply(id: number, ok: boolean, value?: unknown, error?: string) {
    iframe.contentWindow?.postMessage(
      { __pc: true, kind: "capability-result", id, ok, value, error },
      "*",
    );
  }

  async function handleCapability(m: any) {
    if (!whitelist.has(m.name)) {
      reply(m.id, false, undefined, `permission denied: ${m.name}`);
      return;
    }
    const impl = capabilities[m.name];
    if (!impl) {
      reply(m.id, false, undefined, `unknown capability: ${m.name}`);
      return;
    }
    try {
      const value = await impl(m.args || {});
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
