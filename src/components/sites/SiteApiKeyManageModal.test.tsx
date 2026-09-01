import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { resetBrowserMock } from "@/lib/browserMock";
import { useSiteStore } from "@/stores";
import { resetQuotaInflight } from "@/stores/siteStore";
import { SiteApiKeyManageModal } from "./SiteApiKeyManageModal";
import "@/i18n";

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

describe("SiteApiKeyManageModal", () => {
  const writeText = vi.fn().mockResolvedValue(undefined);

  beforeEach(() => {
    resetBrowserMock();
    resetQuotaInflight();
    writeText.mockClear();
    Object.assign(navigator, { clipboard: { writeText } });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("puts copy first in the key actions and copies the decrypted secret", async () => {
    const created = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-one-secret",
    });
    const site = useSiteStore.getState().sites.find((item) => item.id === created.id) ?? created;

    render(
      <Wrapper>
        <SiteApiKeyManageModal open site={site} onClose={() => undefined} />
      </Wrapper>,
    );

    const copy = screen.getByRole("button", { name: /复\s*制/ });
    const rename = screen.getByRole("button", { name: "重命名" });
    expect(copy.compareDocumentPosition(rename) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

    fireEvent.click(copy);
    await waitFor(() => expect(writeText).toHaveBeenCalledWith("sk-one-secret"));
    expect(await screen.findByText("已复制")).toBeInTheDocument();
  });
});
