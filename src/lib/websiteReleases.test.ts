import { afterEach, describe, expect, it, vi } from "vitest";
import {
  GITHUB_API_LATEST,
  githubReleaseApiUrl,
  loadLatestRelease,
} from "./githubRelease";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});

describe("githubReleaseApiUrl", () => {
  it("uses /releases/latest when no tag is provided", () => {
    expect(githubReleaseApiUrl()).toBe(GITHUB_API_LATEST);
    expect(githubReleaseApiUrl("")).toBe(GITHUB_API_LATEST);
    expect(githubReleaseApiUrl("   ")).toBe(GITHUB_API_LATEST);
  });

  it("uses /releases/tags/{tag} so a post-release website build can pin the new version", () => {
    expect(githubReleaseApiUrl("v0.0.6")).toBe(
      "https://api.github.com/repos/doMySelfZy/xiaobai-switch/releases/tags/v0.0.6",
    );
  });

  it("strips refs/tags/ from workflow_run head_branch", () => {
    expect(githubReleaseApiUrl("refs/tags/v0.0.6")).toBe(
      "https://api.github.com/repos/doMySelfZy/xiaobai-switch/releases/tags/v0.0.6",
    );
  });
});

describe("loadLatestRelease", () => {
  it("requests the tagged release when a tag is passed", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        tag_name: "v0.0.6",
        draft: false,
        assets: [
          {
            name: "XiaoBaiSwitch_0.0.6_aarch64.dmg",
            browser_download_url:
              "https://github.com/doMySelfZy/xiaobai-switch/releases/download/v0.0.6/XiaoBaiSwitch_0.0.6_aarch64.dmg",
          },
        ],
      }),
    });
    vi.stubGlobal("fetch", fetchMock);

    const latest = await loadLatestRelease("token", "v0.0.6");

    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.github.com/repos/doMySelfZy/xiaobai-switch/releases/tags/v0.0.6",
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: "Bearer token",
        }),
      }),
    );
    expect(latest?.tag).toBe("v0.0.6");
    expect(latest?.assets["mac-arm"]?.url).toContain("v0.0.6");
  });

  it("does not bake a draft release into the download page", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ tag_name: "v0.0.6", draft: true, assets: [] }),
      }),
    );

    await expect(loadLatestRelease(undefined, "v0.0.6")).rejects.toThrow(
      /draft/i,
    );
  });

  it("fails CI instead of shipping a download page without installer assets", async () => {
    vi.stubEnv("CI", "true");
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ tag_name: "v0.0.6", draft: false, assets: [] }),
      }),
    );

    await expect(loadLatestRelease("token")).rejects.toThrow(/asset/i);
  });
});
