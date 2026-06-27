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
