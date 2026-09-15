import { describe, expect, it } from "vitest";

const signingModule =
  // @ts-expect-error The workflow helper is native ESM without TypeScript declarations.
  await import("../../scripts/validate-updater-signing-secret.mjs");
const { validateUpdaterSigningSecrets } = signingModule;

const RAW_KEY = "untrusted comment: rsign encrypted secret key\nRWRTY0IydGVzdA==\n";
const BASE64_KEY = Buffer.from(RAW_KEY, "utf8").toString("base64");

describe("validateUpdaterSigningSecrets", () => {
  it("rejects the empty key that produced Missing comment in secret key", () => {
    expect(() =>
      validateUpdaterSigningSecrets({ privateKey: "", password: "secret" }),
    ).toThrow(/TAURI_SIGNING_PRIVATE_KEY is empty/);
    expect(() =>
      validateUpdaterSigningSecrets({ privateKey: "   ", password: "secret" }),
    ).toThrow(/TAURI_SIGNING_PRIVATE_KEY is empty/);
  });

  it("rejects a missing password", () => {
    expect(() =>
      validateUpdaterSigningSecrets({ privateKey: BASE64_KEY, password: "" }),
    ).toThrow(/TAURI_SIGNING_PRIVATE_KEY_PASSWORD is empty/);
    expect(() =>
      validateUpdaterSigningSecrets({ privateKey: BASE64_KEY }),
    ).toThrow(/TAURI_SIGNING_PRIVATE_KEY_PASSWORD is empty/);
  });

  it("rejects a key that is not a minisign secret", () => {
    expect(() =>
      validateUpdaterSigningSecrets({
        privateKey: ["test", "invalid", "key"].join("-"),
        password: "secret",
      }),
    ).toThrow(/missing the minisign untrusted comment line/);
  });

  it("accepts raw and base64-encoded signer keys", () => {
    expect(
      validateUpdaterSigningSecrets({ privateKey: RAW_KEY, password: "secret" }),
    ).toEqual({ format: "raw" });
    expect(
      validateUpdaterSigningSecrets({
        privateKey: BASE64_KEY,
        password: "secret",
      }),
    ).toEqual({ format: "base64" });
  });
});
