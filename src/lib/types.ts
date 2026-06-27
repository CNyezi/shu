export type AppEntry = { name: string; path: string };

export type Trigger = { kind: "keyword" | "regex" | "content"; value: string };

export type Feature = {
  code: string;
  type: "ui" | "logic";
  entry: string;
  triggers: Trigger[];
};

export type Plugin = {
  id: string;
  name: string;
  version: string;
  icon?: string;
  features: Feature[];
  permissions: string[];
  /** folder name on disk, injected by the Rust loader */
  _dir: string;
  /** permissions actually granted by the user (bundled = all) */
  granted: string[];
  /** "bundled" | "installed" */
  source: string;
};

export type InstalledPlugin = {
  id: string;
  version: string;
  granted: string[];
  source: string;
  origin: string;
};

export type PackageInspect = {
  manifest: {
    id: string;
    name: string;
    version: string;
    icon?: string;
    permissions: string[];
  };
  sha256: string;
  is_upgrade: boolean;
  new_permissions: string[];
};

export type RegistryPlugin = {
  id: string;
  name: string;
  version: string;
  description: string;
  permissions: string[];
  packageUrl: string;
  sha256: string;
};

export type RegistryFeed = {
  version: 1;
  plugins: RegistryPlugin[];
};

export type ResultItem =
  | { kind: "app"; title: string; subtitle: string; path: string }
  | { kind: "feature"; title: string; subtitle: string; plugin: Plugin; feature: Feature };
