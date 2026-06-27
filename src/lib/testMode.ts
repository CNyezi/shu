export const TEST_PATH = "/test";
export const DEFAULT_TEST_PACKAGE = "/tmp/shu-json-preview.pcp";

export function isTestPath(pathname: string, dev: boolean): boolean {
  return dev && pathname === TEST_PATH;
}

export function testPackagePath(search: string): string {
  return new URLSearchParams(search).get("package") || DEFAULT_TEST_PACKAGE;
}
