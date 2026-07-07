export type MatchTarget = { name: string; pinyin?: string | null; initials?: string | null };

function isSubsequence(q: string, s: string): boolean {
  let i = 0;
  for (const ch of s) {
    if (ch === q[i]) i++;
    if (i >= q.length) return true;
  }
  return false;
}

function wordInitials(name: string): string {
  return name
    .split(/[\s\-_./]+/)
    .filter(Boolean)
    .map((w) => w[0])
    .join("")
    .toLowerCase();
}

/** 0 = 不匹配；分数越高越靠前。分档保证排序稳定、可测试。 */
export function matchScore(query: string, t: MatchTarget): number {
  const q = query.toLowerCase();
  if (!q) return 0;
  const name = t.name.toLowerCase();
  if (name.startsWith(q)) return 100;
  if (wordInitials(t.name).startsWith(q)) return 90;
  if (name.includes(q)) return 80;
  if (t.initials?.startsWith(q)) return 75; // 拼音首字母: wx -> 微信
  if (t.pinyin?.startsWith(q)) return 70; // 全拼前缀: weixin
  if (t.pinyin?.includes(q)) return 60;
  if (q.length >= 2 && isSubsequence(q, name)) return 40;
  if (q.length >= 2 && t.pinyin && isSubsequence(q, t.pinyin)) return 30;
  return 0;
}
