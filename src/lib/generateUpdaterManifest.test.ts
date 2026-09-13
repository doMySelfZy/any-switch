import { describe, expect, it } from "vitest";

const manifestModule =
  // @ts-expect-error The workflow helper is native ESM without TypeScript declarations.
  await import("../../scripts/generate-updater-manifest.mjs");
const {
  buildUpdaterManifest,
  classifySignature,
  updaterNotesFromReleaseBody,
} = manifestModule;

function assetPair(name: string, id: number) {
  return [
    { id, name },
    { id: id + 1, name: `${name}.sig` },
  ];
}

const assets = [
  ...assetPair("XiaoBaiSwitchPlus_aarch64.app.tar.gz", 1),
  ...assetPair("XiaoBaiSwitchPlus_x64.app.tar.gz", 3),
  ...assetPair("XiaoBaiSwitchPlus_0.0.1_arm64-setup.exe", 5),
  ...assetPair("XiaoBaiSwitchPlus_0.0.1_x64_en-US.msi", 7),
  ...assetPair("XiaoBaiSwitchPlus_0.0.1_x64-setup.exe", 9),
];

const signatures = new Map(
  assets
    .filter(({ name }) => name.endsWith(".sig"))
    .map(({ name }) => [name, `signature:${name}`]),
);

describe("updater notes from release body", () => {
  it("keeps changelog and drops the install footer after the marker", () => {
    expect(
      updaterNotesFromReleaseBody(`## 更新内容

### 🐛 Bug 修复
- **updater**: 检测更新走应用代理规则

<!-- updater-notes-end -->

## 下载
- **macOS（Apple Silicon）**: \`.dmg\`
`),
    ).toBe(`## 更新内容

### 🐛 Bug 修复
- **updater**: 检测更新走应用代理规则`);
  });

  it("returns the whole body when the marker is missing", () => {
    expect(updaterNotesFromReleaseBody("just notes")).toBe("just notes");
    expect(updaterNotesFromReleaseBody("")).toBe("");
  });
});

describe("updater manifest generation", () => {
  it("classifies supported updater signature assets", () => {
    expect(classifySignature("XiaoBaiSwitchPlus_aarch64.app.tar.gz.sig")).toEqual({
      os: "darwin",
      arch: "aarch64",
      bundle: "app",
    });
    expect(classifySignature("XiaoBaiSwitchPlus_0.0.1_x64-setup.exe.sig")).toEqual({
      os: "windows",
      arch: "x86_64",
      bundle: "nsis",
    });
    expect(classifySignature("XiaoBaiSwitchPlus_v0.0.1_windows-x64-portable.zip.sig")).toBeNull();
  });

  it("builds one complete updater manifest after all platform uploads", () => {
    const manifest = buildUpdaterManifest({
      version: "0.0.1",
      notes: "release notes",
      pubDate: "2026-08-19T15:00:00.000Z",
      repository: "doMySelfZy/xiaobai-switch-plus",
      serverUrl: "https://github.com",
      tag: "v0.0.1",
      assets,
      signatures,
    });

    expect(manifest.version).toBe("0.0.1");
    expect(manifest.notes).toBe("release notes");
    expect(manifest.platforms["windows-aarch64"].url).toBe(
      "https://github.com/doMySelfZy/xiaobai-switch-plus/releases/download/v0.0.1/XiaoBaiSwitchPlus_0.0.1_arm64-setup.exe",
    );
    expect(manifest.platforms["windows-aarch64-nsis"].signature).toBe(
      "signature:XiaoBaiSwitchPlus_0.0.1_arm64-setup.exe.sig",
    );
    expect(manifest.platforms["darwin-x86_64-app"].url).toBe(
      "https://github.com/doMySelfZy/xiaobai-switch-plus/releases/download/v0.0.1/XiaoBaiSwitchPlus_x64.app.tar.gz",
    );
    expect(manifest.platforms["windows-x86_64"].url).toBe(
      "https://github.com/doMySelfZy/xiaobai-switch-plus/releases/download/v0.0.1/XiaoBaiSwitchPlus_0.0.1_x64_en-US.msi",
    );
    expect(Object.keys(manifest.platforms)).toHaveLength(9);
  });

  it("fails instead of publishing an incomplete updater manifest", () => {
    expect(() =>
      buildUpdaterManifest({
        version: "0.0.1",
        notes: "",
        pubDate: "2026-08-19T15:00:00.000Z",
        repository: "doMySelfZy/xiaobai-switch-plus",
        serverUrl: "https://github.com",
        tag: "v0.0.1",
        assets: assets.filter(({ name }) => !name.includes("XiaoBaiSwitchPlus_x64.app.tar.gz")),
        signatures,
      }),
    ).toThrow("missing updater platforms: darwin-x86_64, darwin-x86_64-app");
  });
});
