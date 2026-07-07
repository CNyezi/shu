<script lang="ts">
  import { DEFAULT_HOTKEY, setHotkey } from "./settings";

  let {
    hotkey,
    onSaved,
    onError,
  }: {
    hotkey: string;
    onSaved: (hotkey: string) => void;
    onError: (msg: string) => void;
  } = $props();

  let recorded = $state("");
  let recording = $state(false);

  const MOD_KEYS = new Set(["Meta", "Control", "Alt", "Shift"]);

  function keyFromCode(code: string): string {
    if (code.startsWith("Key")) return code.slice(3).toLowerCase();
    if (code.startsWith("Digit")) return code.slice(5);
    return code.toLowerCase(); // Space / Comma / F1 …
  }

  function capture(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (MOD_KEYS.has(e.key)) return; // 只按修饰键时等待主键
    const mods = [
      e.metaKey ? "super" : "",
      e.ctrlKey ? "ctrl" : "",
      e.altKey ? "alt" : "",
      e.shiftKey ? "shift" : "",
    ].filter(Boolean);
    if (mods.length === 0) return; // 全局热键必须带修饰键
    recorded = [...mods, keyFromCode(e.code)].join("+");
    recording = false;
  }

  async function save() {
    if (!recorded) return;
    try {
      await setHotkey(recorded);
      onSaved(recorded);
      recorded = "";
    } catch (err) {
      onError(String(err));
    }
  }
</script>

<div class="settings">
  <div class="row">
    <span class="name">唤起热键</span>
    <input
      readonly
      value={recording ? "按下新快捷键…" : recorded || hotkey}
      onfocus={() => (recording = true)}
      onblur={() => (recording = false)}
      onkeydown={capture}
    />
    <button disabled={!recorded} onclick={save}>保存</button>
  </div>
  <div class="hint">点击输入框后按下组合键（需包含 ⌘/⌃/⌥ 修饰键）。默认 {DEFAULT_HOTKEY}。</div>
</div>

<style>
  .settings {
    padding: 12px 14px;
    color: #e8e8ea;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .name {
    font-size: 13px;
    white-space: nowrap;
  }
  input {
    flex: 1;
    border: 0;
    outline: 0;
    background: rgba(255, 255, 255, 0.06);
    color: #fff;
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 13px;
  }
  input:focus {
    outline: 1px solid var(--sel);
  }
  button {
    border: 0;
    background: var(--sel);
    color: #fff;
    border-radius: 6px;
    padding: 6px 12px;
    cursor: pointer;
    font-size: 13px;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .hint {
    margin-top: 8px;
    color: var(--muted);
    font-size: 12px;
  }
</style>
