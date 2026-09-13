import { describe, expect, it } from "vitest";
import { buildSiteDeepLink, parseSiteDeepLink } from "./siteDeepLink";

describe("parseSiteDeepLink", () => {
  it("parses multi-route payload from repeated baseurls", () => {
    const payload = parseSiteDeepLink(
      "anyswitch://sites?name=Example%20Relay&baseurls=https%3A%2F%2Fa.example.com%2Fv1&baseurls=https%3A%2F%2Fb.example.com%2Fv1&apikey=sk-test&protocol=openai_compatible&notes=hi",
    );
    expect(payload).toEqual({
      name: "Example Relay",
      baseUrls: ["https://a.example.com/v1", "https://b.example.com/v1"],
      apiKey: "sk-test",
      protocol: "openai_compatible",
      notes: "hi",
      capabilities: {},
      hasCapabilityParams: false,
      keyName: null,
    });
  });

  it("parses triple-slash sites URL", () => {
    const payload = parseSiteDeepLink(
      "anyswitch:///sites?name=Claude&baseurls=https%3A%2F%2Fapi.anthropic.com&apikey=sk-ant&protocol=anthropic",
    );
    expect(payload).toEqual({
      name: "Claude",
      baseUrls: ["https://api.anthropic.com"],
      apiKey: "sk-ant",
      protocol: "anthropic",
      notes: null,
      capabilities: {},
      hasCapabilityParams: false,
      keyName: null,
    });
  });

  it("accepts comma-separated baseurls", () => {
    const payload = parseSiteDeepLink(
      "anyswitch://sites?name=xxx&baseurls=https://a.example.com,https://b.example.com",
    );
    expect(payload?.baseUrls).toEqual(["https://a.example.com", "https://b.example.com"]);
    expect(payload?.apiKey).toBeNull();
    expect(payload?.protocol).toBe("openai_compatible");
  });

  it("accepts pipe-separated baseurls", () => {
    const payload = parseSiteDeepLink(
      "anyswitch://sites?name=xxx&baseurls=https://a.example.com|https://b.example.com",
    );
    expect(payload?.baseUrls).toEqual(["https://a.example.com", "https://b.example.com"]);
  });

  it("merges baseurl and baseurls in query order", () => {
    const payload = parseSiteDeepLink(
      "anyswitch://sites?name=Mix&baseurl=https://first.example.com&baseurls=https://second.example.com",
    );
    expect(payload?.baseUrls).toEqual([
      "https://first.example.com",
      "https://second.example.com",
    ]);
  });

  it("maps protocol aliases including AQBot type=", () => {
    expect(
      parseSiteDeepLink(
        "anyswitch://sites?name=O&baseurls=https://a.example.com&type=openai",
      )?.protocol,
    ).toBe("openai_compatible");
    expect(
      parseSiteDeepLink(
        "anyswitch://sites?name=A&baseurls=https://a.example.com&protocol=anthropic",
      )?.protocol,
    ).toBe("anthropic");
  });

  it("returns null for other schemes, targets, or missing name", () => {
    expect(
      parseSiteDeepLink(
        "aqbot://providers?name=x&baseurl=https://a.com&apikey=k&type=openai",
      ),
    ).toBeNull();
    expect(parseSiteDeepLink("https://example.com")).toBeNull();
    expect(parseSiteDeepLink("anyswitch://apply?name=x")).toBeNull();
    expect(parseSiteDeepLink("anyswitch://sites?baseurls=https://a.example.com")).toBeNull();
  });

  it("returns null for invalid routes or protocol", () => {
    expect(
      parseSiteDeepLink("anyswitch://sites?name=x&baseurls=ftp://a.example.com"),
    ).toBeNull();
    expect(parseSiteDeepLink("anyswitch://sites?name=x")).toBeNull();
    expect(
      parseSiteDeepLink(
        "anyswitch://sites?name=x&baseurls=https://a.example.com&protocol=gemini",
      ),
    ).toBeNull();
  });

  it("round-trips through buildSiteDeepLink", () => {
    const built = buildSiteDeepLink({
      name: "Relay",
      baseUrls: ["https://a.example.com", "https://b.example.com"],
      apiKey: "sk-test",
      protocol: "anthropic",
      notes: "n",
      capabilities: {},
      hasCapabilityParams: false,
      keyName: null,
    });
    expect(parseSiteDeepLink(built)).toEqual({
      name: "Relay",
      baseUrls: ["https://a.example.com", "https://b.example.com"],
      apiKey: "sk-test",
      protocol: "anthropic",
      notes: "n",
      capabilities: {},
      hasCapabilityParams: false,
      keyName: null,
    });
  });

  it("parses Codex capability presets and fills missing known keys", () => {
    const payload = parseSiteDeepLink(
      "anyswitch://sites?name=Relay&baseurls=https://a.example.com&codex-compact=1&codex-vision=1",
    );
    expect(payload?.hasCapabilityParams).toBe(true);
    expect(payload?.capabilities).toEqual({
      "codex-compact": true,
      "codex-vision": true,
      "codex-imagegen": false,
      "codex-search": false,
    });
  });

  it("round-trips enabled capability flags", () => {
    const built = buildSiteDeepLink({
      name: "Relay",
      baseUrls: ["https://a.example.com"],
      apiKey: null,
      protocol: "openai_compatible",
      notes: null,
      capabilities: {
        "codex-compact": true,
        "codex-vision": true,
        "codex-imagegen": false,
        "codex-search": false,
      },
      hasCapabilityParams: true,
      keyName: null,
    });
    expect(built).toContain("codex-compact=1");
    expect(built).toContain("codex-vision=1");
    expect(built).not.toContain("codex-imagegen");
    expect(parseSiteDeepLink(built)?.capabilities["codex-compact"]).toBe(true);
    expect(parseSiteDeepLink(built)?.capabilities["codex-search"]).toBe(false);
  });
});
