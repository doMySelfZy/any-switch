import { afterEach, describe, expect, it } from "vitest";
import {
  consumeStartupDeepLinkUrls,
  rememberHandledDeepLink,
  resetDeepLinkSession,
} from "./deepLinkSession";

const SCHEME = "xiaobaiswitchplus://sites?name=SchemeTest&baseurls=https://a.example.com";

describe("deepLinkSession", () => {
  afterEach(() => {
    resetDeepLinkSession();
  });

  it("lets a startup URL through once and swallows it on reload", () => {
    expect(consumeStartupDeepLinkUrls([SCHEME])).toEqual([SCHEME]);
    expect(consumeStartupDeepLinkUrls([SCHEME])).toEqual([]);
  });

  it("skips getCurrent after the same URL was handled from onOpenUrl", () => {
    rememberHandledDeepLink(SCHEME);
    expect(consumeStartupDeepLinkUrls([SCHEME])).toEqual([]);
  });

  it("still delivers a different import link", () => {
    expect(consumeStartupDeepLinkUrls([SCHEME])).toEqual([SCHEME]);
    const other = "xiaobaiswitchplus://sites?name=Other&baseurls=https://b.example.com";
    expect(consumeStartupDeepLinkUrls([other])).toEqual([other]);
  });
});
