<script lang="ts">
  import { permissionLabel, isHighRisk, isFsRead } from "./permissions";
  import type { PackageInspect } from "./types";

  let {
    info,
    onApprove,
    onCancel,
  }: {
    info: PackageInspect;
    onApprove: () => void;
    onCancel: () => void;
  } = $props();

  const perms = $derived(info.manifest.permissions ?? []);
  const isNew = (p: string) => info.is_upgrade && info.new_permissions.includes(p);
  const hasHighRisk = $derived(perms.some(isHighRisk));
  // The dangerous combo: can read your files AND reach the network = can upload them.
  const exfilCombo = $derived(perms.some(isFsRead) && perms.includes("network"));

  let acknowledged = $state(false);
  const canInstall = $derived(!hasHighRisk || acknowledged);
</script>

<div class="consent">
  <div class="head">
    <div class="title">{info.manifest.name}</div>
    <div class="ver">v{info.manifest.version}{info.is_upgrade ? "（升级）" : ""}</div>
  </div>
  <div class="id">{info.manifest.id}</div>

  <div class="section">该插件申请以下能力：</div>
  <ul class="perms">
    {#each perms as p (p)}
      <li class:fresh={isNew(p)} class:risk={isHighRisk(p)}>
        {#if isHighRisk(p)}⚠️ {/if}{permissionLabel(p)}{isNew(p) ? "  · 新增" : ""}
      </li>
    {/each}
    {#if perms.length === 0}
      <li class="none">无需任何系统能力</li>
    {/if}
  </ul>

  {#if exfilCombo}
    <div class="warn">⚠️ 该插件能<b>读取你的文件</b>并<b>联网</b>——它有能力把你的数据上传到任意服务器。请确认你信任它。</div>
  {:else if hasHighRisk}
    <div class="warn">⚠️ 该插件申请了高危权限（上方标红项）。请确认这些权限对它的用途是合理的。</div>
  {/if}

  <div class="hash">SHA-256: {info.sha256}</div>

  {#if hasHighRisk}
    <label class="ack">
      <input type="checkbox" bind:checked={acknowledged} />
      我已了解上述高危权限的风险
    </label>
  {/if}

  <div class="actions">
    <button class="cancel" onclick={onCancel}>取消</button>
    <button class="ok" disabled={!canInstall} onclick={onApprove}>
      {info.is_upgrade ? "升级并授权" : "安装并授权"}
    </button>
  </div>
</div>

<style>
  .consent {
    padding: 14px 16px;
    color: #e8e8ea;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 12px;
  }
  .title {
    font-size: 16px;
    font-weight: 600;
  }
  .ver {
    color: var(--muted);
    font-size: 12px;
  }
  .id {
    font-size: 11px;
    color: var(--muted);
    word-break: break-all;
    margin-bottom: 12px;
  }
  .section {
    font-size: 13px;
    color: var(--muted);
    margin-bottom: 6px;
  }
  .perms {
    list-style: none;
    margin: 0 0 12px;
    padding: 0;
  }
  .perms li {
    padding: 5px 10px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.05);
    margin-bottom: 4px;
    font-size: 13px;
  }
  .perms li.fresh {
    background: rgba(47, 111, 237, 0.25);
  }
  .perms li.risk {
    background: rgba(229, 112, 122, 0.18);
    color: #ff9aa2;
  }
  .perms li.none {
    color: var(--muted);
    background: none;
  }
  .warn {
    font-size: 12px;
    line-height: 1.5;
    color: #ffb4ba;
    background: rgba(229, 112, 122, 0.12);
    border: 1px solid rgba(229, 112, 122, 0.3);
    border-radius: 7px;
    padding: 8px 10px;
    margin-bottom: 12px;
  }
  .warn b {
    color: #ff7a85;
  }
  .hash {
    font-size: 11px;
    color: var(--muted);
    word-break: break-all;
    margin-bottom: 12px;
  }
  .ack {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
    color: #ffb4ba;
    margin-bottom: 14px;
    cursor: pointer;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  button {
    border: 0;
    border-radius: 6px;
    padding: 6px 14px;
    cursor: pointer;
    font-size: 13px;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .cancel {
    background: #3a3a3e;
    color: #e8e8ea;
  }
  .ok {
    background: var(--sel);
    color: #fff;
  }
</style>
