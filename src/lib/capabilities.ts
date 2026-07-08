export type PermissionTier = "high" | "sensitive" | "normal";

export const CAPABILITIES: Record<
  string,
  { label: string; tier?: PermissionTier; permission?: string }
> = {
  "clipboard.read": { label: "读取剪贴板" },
  "clipboard.write": { label: "写入剪贴板" },
  "clipboard.readImage": { label: "读取剪贴板图片" },
  "clipboard.writeImage": { label: "写入剪贴板图片" },
  "clipboard.readFiles": { label: "读取剪贴板中的文件" },
  "clipboard.writeFiles": { label: "复制文件到剪贴板" },
  "shell.openUrl": { label: "用浏览器打开网址" },
  "shell.openPath": { label: "用默认程序打开文件" },
  "hosts.read": { label: "读取 hosts 文件" },
  "hosts.write": { label: "修改 hosts 文件（需要管理员权限）", tier: "high" },
  notification: { label: "发送系统通知" },
  network: { label: "访问网络（可连接任意服务器）", tier: "high" },
  "network.http": { label: "访问网络（可连接任意服务器）", permission: "network" },
  "image.compress": { label: "压缩图片", tier: "normal" },
  "dialog.saveFile": { label: "弹出保存对话框并写入文件", tier: "normal" },
};

const FS_SCOPES: Record<string, string> = {
  downloads: "下载目录",
  desktop: "桌面",
  documents: "文稿目录",
  temp: "临时目录",
  home: "整个用户目录",
};

export function capabilityPermission(id: string): string {
  return CAPABILITIES[id]?.permission ?? id;
}

export function effectivePermissions(declared: string[] = [], granted: string[] = []): string[] {
  const declaredSet = new Set(declared);
  return granted.filter((p) => declaredSet.has(p));
}

export function canUseCapability(
  declared: string[] = [],
  granted: string[] = [],
  capability: string,
): boolean {
  return new Set(effectivePermissions(declared, granted)).has(capabilityPermission(capability));
}

export function permissionLabel(id: string): string {
  const m = id.match(/^fs\.(\w+)\.(read|write)$/);
  if (m) {
    const scope = FS_SCOPES[m[1]] ?? m[1];
    return `${m[2] === "read" ? "读取" : "写入 / 删除"} ${scope}`;
  }
  return CAPABILITIES[id]?.label ?? id;
}

export function isFsRead(id: string): boolean {
  return /^fs\.\w+\.(read|write)$/.test(id);
}

export function permissionTier(id: string): PermissionTier {
  if (CAPABILITIES[id]?.tier) return CAPABILITIES[id].tier;
  if (id === "fs.home.read" || id === "fs.home.write") return "high";
  if (/^fs\.\w+\.write$/.test(id)) return "high";
  if (/^fs\.\w+\.read$/.test(id)) return "sensitive";
  return "normal";
}
