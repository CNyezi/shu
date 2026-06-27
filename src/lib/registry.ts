export const OFFICIAL_REGISTRY_URL = ((import.meta as any).env?.VITE_SHU_OFFICIAL_REGISTRY_URL || "").trim();

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
