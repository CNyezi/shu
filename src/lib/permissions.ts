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
  "fs.read": "读取你电脑上的文件",
  "fs.write": "写入 / 删除你电脑上的文件",
  notification: "发送系统通知",
  network: "访问网络（可连接任意服务器）",
};

// Permissions that can do real damage and deserve a louder warning at install.
const HIGH_RISK = new Set(["fs.read", "fs.write", "network", "hosts.write"]);

export function permissionLabel(id: string): string {
  return LABELS[id] ?? id;
}

export function isHighRisk(id: string): boolean {
  return HIGH_RISK.has(id);
}
