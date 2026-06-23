const LABELS: Record<string, string> = {
  "clipboard.read": "读取剪贴板",
  "clipboard.write": "写入剪贴板",
  "shell.openUrl": "用浏览器打开网址",
  "shell.openPath": "用默认程序打开文件",
};

export function permissionLabel(id: string): string {
  return LABELS[id] ?? id;
}
