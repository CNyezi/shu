const LABELS: Record<string, string> = {
  "clipboard.read": "读取剪贴板",
  "clipboard.write": "写入剪贴板",
  "clipboard.readImage": "读取剪贴板图片",
  "clipboard.writeImage": "写入剪贴板图片",
  "clipboard.readFiles": "读取剪贴板中的文件",
  "clipboard.writeFiles": "复制文件到剪贴板",
  "shell.openUrl": "用浏览器打开网址",
  "shell.openPath": "用默认程序打开文件",
  "hosts.read": "读取 hosts 文件",
  "hosts.write": "修改 hosts 文件（需要管理员权限）",
  notification: "发送系统通知",
  network: "访问网络（可连接任意服务器）",
};

// fs scope -> human name (permissions look like `fs.downloads.read`).
const FS_SCOPES: Record<string, string> = {
  downloads: "下载目录",
  desktop: "桌面",
  documents: "文稿目录",
  temp: "临时目录",
  home: "整个用户目录",
};

export function permissionLabel(id: string): string {
  const m = id.match(/^fs\.(\w+)\.(read|write)$/);
  if (m) {
    const scope = FS_SCOPES[m[1]] ?? m[1];
    return `${m[2] === "read" ? "读取" : "写入 / 删除"} ${scope}`;
  }
  return LABELS[id] ?? id;
}

// Permissions that can do real damage and deserve a louder warning at install:
// network, hosts.write, anything touching the whole home dir, and any fs write.
export function isHighRisk(id: string): boolean {
  if (id === "network" || id === "hosts.write") return true;
  if (id === "fs.home.read" || id === "fs.home.write") return true;
  if (/^fs\.\w+\.write$/.test(id)) return true;
  return false;
}

// Reading any scope + network = potential exfiltration.
export function isFsRead(id: string): boolean {
  return /^fs\.\w+\.(read|write)$/.test(id);
}

// Three tiers for the consent dialog. "high" = red + ⚠️; "sensitive" = amber
// (any scoped file read); both require the acknowledgement checkbox.
export function permissionTier(id: string): "high" | "sensitive" | "normal" {
  if (isHighRisk(id)) return "high";
  if (/^fs\.\w+\.read$/.test(id)) return "sensitive";
  return "normal";
}
