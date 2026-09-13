import { describe, expect, it } from "vitest";
import { parseProxyHeadersJson } from "@/components/sites/ProxyHeaderEditor";

describe("parseProxyHeadersJson", () => {
  it("treats empty input as no overrides", () => {
    expect(parseProxyHeadersJson("")).toEqual({ headers: [] });
    expect(parseProxyHeadersJson("   \n ")).toEqual({ headers: [] });
  });

  it("accepts the array form with explicit enabled flags", () => {
    const result = parseProxyHeadersJson(
      JSON.stringify([
        { name: "x-opencode-session", value: "${SESSION}", enabled: true },
        { name: "X-Gone", value: "1", enabled: false },
      ]),
    );
    expect(result.error).toBeUndefined();
    expect(result.headers).toHaveLength(2);
    expect(result.headers?.[1]).toEqual({ name: "X-Gone", value: "1", enabled: false });
  });

  it("accepts the shorthand object form and enables entries by default", () => {
    const result = parseProxyHeadersJson('{"User-Agent":"my-agent/1.0"}');
    expect(result.error).toBeUndefined();
    expect(result.headers).toEqual([
      { name: "User-Agent", value: "my-agent/1.0", enabled: true },
    ]);
  });

  it("rejects malformed JSON and non-object payloads", () => {
    expect(parseProxyHeadersJson("{").error).toBeTruthy();
    expect(parseProxyHeadersJson('"just a string"').error).toBeTruthy();
    expect(parseProxyHeadersJson("null").error).toBeTruthy();
  });

  it("rejects invalid header names (matching the Rust-side token rule)", () => {
    expect(parseProxyHeadersJson(JSON.stringify([{ name: "X Foo", value: "v" }])).error).toBeTruthy();
    expect(parseProxyHeadersJson(JSON.stringify([{ name: "Authorization:", value: "v" }])).error).toBeTruthy();
    expect(parseProxyHeadersJson(JSON.stringify([{ name: "  ", value: "v" }])).error).toBeTruthy();
  });

  it("rejects control characters in values", () => {
    const result = parseProxyHeadersJson(
      JSON.stringify([{ name: "X-Foo", value: "bad\nvalue" }]),
    );
    expect(result.error).toBeTruthy();
  });

  it("allows overriding authorization (the point of the feature)", () => {
    const result = parseProxyHeadersJson(
      JSON.stringify([{ name: "authorization", value: "Bearer ${API_KEY}" }]),
    );
    expect(result.error).toBeUndefined();
    expect(result.headers?.[0]?.name).toBe("authorization");
  });

  it("rejects protected transport headers", () => {
    for (const name of ["host", "Content-Length", "connection", "content-type"]) {
      const result = parseProxyHeadersJson(JSON.stringify([{ name, value: "x" }]));
      expect(result.error, `expected ${name} to be rejected`).toBeTruthy();
    }
  });

  it("rejects case-insensitive duplicates instead of picking one at random", () => {
    const result = parseProxyHeadersJson(
      JSON.stringify([
        { name: "X-Foo", value: "a" },
        { name: "x-foo", value: "b" },
      ]),
    );
    expect(result.error).toBeTruthy();
  });
});
