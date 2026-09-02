import { fireEvent, render, screen } from "@testing-library/react";
import { App, ConfigProvider } from "antd";
import { describe, expect, it } from "vitest";
import { resetBrowserMock } from "@/lib/browserMock";
import { useSiteStore } from "@/stores";
import { resetQuotaInflight } from "@/stores/siteStore";
import { SiteApiKeySwitcher } from "./SiteApiKeySwitcher";
import "@/i18n";

function Wrapper({ children }: { children: React.ReactNode }) {
  return (
    <ConfigProvider>
      <App>{children}</App>
    </ConfigProvider>
  );
}

describe("SiteApiKeySwitcher", () => {
  it("lists keys and can open manage dialog", async () => {
    resetBrowserMock();
    resetQuotaInflight();
    useSiteStore.setState({
      sites: [],
      modelsBySite: {},
      fetchingModels: false,
      fetchingModelsByKey: {},
    });
    const site = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-one",
    });
    await useSiteStore.getState().addApiKey(site.id, { apiKey: "sk-two", label: "K 2" });
    const current = useSiteStore.getState().sites[0]!;

    render(
      <Wrapper>
        <SiteApiKeySwitcher site={current} />
      </Wrapper>,
    );

    expect(screen.getByRole("button", { name: "管理密钥" })).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText("切换密钥"));
    expect(await screen.findByText("K 1")).toBeInTheDocument();
    expect(screen.getByText("K 2")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "管理密钥" }));
    expect(await screen.findByPlaceholderText("sk-...")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "测试密钥" }).length).toBeGreaterThan(0);
  });

  it("opens a closable confirm with spaced actions and no cancel button", async () => {
    resetBrowserMock();
    resetQuotaInflight();
    useSiteStore.setState({
      sites: [],
      modelsBySite: {},
      fetchingModels: false,
      fetchingModelsByKey: {},
    });
    const site = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-one",
    });
    await useSiteStore.getState().addApiKey(site.id, { apiKey: "sk-two", label: "K 2" });
    const current = useSiteStore.getState().sites[0]!;

    render(
      <Wrapper>
        <SiteApiKeySwitcher site={current} />
      </Wrapper>,
    );

    fireEvent.click(screen.getByLabelText("切换密钥"));
    fireEvent.click(await screen.findByText("K 2"));
    expect(await screen.findByText("仅切换站点并刷新模型")).toBeInTheDocument();
    expect(screen.getByText("切换并同步已应用工具")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /取\s*消/ })).not.toBeInTheDocument();
    expect(document.querySelector(".ant-modal-close")).toBeTruthy();
    const footer = document.querySelector(".ant-modal-footer");
    expect(footer).toBeTruthy();
    expect(footer).toHaveTextContent("仅切换站点并刷新模型");
    expect(footer).toHaveTextContent("切换并同步已应用工具");
    const dialog = document.querySelector(".ant-modal");
    expect(dialog).toBeTruthy();
    expect((dialog as HTMLElement).style.width).toBe("560px");
  });
});
