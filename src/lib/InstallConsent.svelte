<script lang="ts">
  import { permissionLabel } from "./permissions";
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
</script>

<div class="consent">
  <div class="head">
    <div class="title">{info.manifest.name}</div>
    <div class="ver">v{info.manifest.version}{info.is_upgrade ? "（升级）" : ""}</div>
  </div>

  <div class="section">该插件申请以下能力：</div>
  <ul class="perms">
    {#each perms as p (p)}
      <li class:fresh={isNew(p)}>
        {permissionLabel(p)}{isNew(p) ? "  · 新增" : ""}
      </li>
    {/each}
    {#if perms.length === 0}
      <li class="none">无需任何系统能力</li>
    {/if}
  </ul>

  <div class="hash">SHA-256: {info.sha256}</div>

  <div class="actions">
    <button class="cancel" onclick={onCancel}>取消</button>
    <button class="ok" onclick={onApprove}>
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
  .perms li.none {
    color: var(--muted);
    background: none;
  }
  .hash {
    font-size: 11px;
    color: var(--muted);
    word-break: break-all;
    margin-bottom: 14px;
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
  .cancel {
    background: #3a3a3e;
    color: #e8e8ea;
  }
  .ok {
    background: var(--sel);
    color: #fff;
  }
</style>
