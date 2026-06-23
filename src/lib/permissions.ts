const LABELS: Record<string, string> = {
  "clipboard.read": "读取剪贴板",
  "clipboard.write": "写入剪贴板",
  "shell.openUrl": "用浏览器打开网址",
  "shell.openPath": "用默认程序打开文件",
  "hosts.read": "读取 hosts 文件",
  "hosts.write": "修改 hosts 文件（需要管理员权限）",
};

export function permissionLabel(id: string): string {
  return LABELS[id] ?? id;
}
