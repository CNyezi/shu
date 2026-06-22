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
};

export type ResultItem =
  | { kind: "app"; title: string; subtitle: string; path: string }
  | { kind: "feature"; title: string; subtitle: string; plugin: Plugin; feature: Feature };
