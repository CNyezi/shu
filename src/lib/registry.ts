export const OFFICIAL_REGISTRY_URL = (
  (import.meta as any).env?.VITE_SHU_OFFICIAL_REGISTRY_URL ||
  "https://raw.githubusercontent.com/CNyezi/shu-registry/main/registry.json"
).trim();

export function registriesWithOfficial(registries: string[], officialUrl = OFFICIAL_REGISTRY_URL): string[] {
  return [...new Set([officialUrl.trim(), ...registries].filter(Boolean))];
}

export function isOfficialRegistry(url: string, officialUrl = OFFICIAL_REGISTRY_URL): boolean {
  return officialUrl.trim() !== "" && url === officialUrl.trim();
}

export function isRegistryFeed(value: unknown): boolean {
  const feed = value as any;
  return (
    feed?.version === 1 &&
    Array.isArray(feed.plugins) &&
    feed.plugins.every((p: any) =>
      typeof p?.id === "string" &&
      typeof p.name === "string" &&
      typeof p.version === "string" &&
      typeof p.description === "string" &&
      Array.isArray(p.permissions) &&
      p.permissions.every((x: any) => typeof x === "string") &&
      typeof p.packageUrl === "string" &&
      /^https?:\/\//i.test(p.packageUrl) &&
      typeof p.sha256 === "string" &&
      /^[0-9a-f]{64}$/i.test(p.sha256)
    )
  );
}

export type PluginRef = { id: string; version: string };

/** 市场条目状态：已装副本优先，捆绑预装也算已安装（可被市场升级遮蔽）。 */
export function regStatus(
  p: PluginRef,
  installed: PluginRef[],
  bundled: PluginRef[],
): "none" | "installed" | "update" {
  const cur = installed.find((i) => i.id === p.id) ?? bundled.find((b) => b.id === p.id);
  if (!cur) return "none";
  return cur.version === p.version ? "installed" : "update";
}
