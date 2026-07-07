export type UseRec = { n: number; t: number };

const HALF_LIFE_MS = 7 * 24 * 3600 * 1000;
const KEY = "shu.frecency.v1";

export function bump(rec: UseRec | undefined, now: number): UseRec {
  return { n: (rec?.n ?? 0) + 1, t: now };
}

export function score(rec: UseRec | undefined, now: number): number {
  if (!rec) return 0;
  return rec.n * Math.pow(0.5, Math.max(0, now - rec.t) / HALF_LIFE_MS);
}

// --- localStorage 薄封装（node 测试只测上面的纯函数） ---
function load(): Record<string, UseRec> {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? "{}");
    return v && typeof v === "object" && !Array.isArray(v) ? v : {};
  } catch {
    return {};
  }
}

export function recordUse(id: string): void {
  const all = load();
  all[id] = bump(all[id], Date.now());
  localStorage.setItem(KEY, JSON.stringify(all));
}

/** 返回一个查分函数（一次加载，避免每个候选项反复读 localStorage）。 */
export function frecencyRanker(): (id: string) => number {
  const all = load();
  const now = Date.now();
  return (id) => score(all[id], now);
}
