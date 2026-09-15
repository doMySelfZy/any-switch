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
    useSiteStore.setState({
      sites: [],
      modelsBySite: {},
      fetchingModels: false,
      fetchingModelsByKey: {},
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("copies the plaintext key from the list row", async () => {
    const created = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: ["test", "key", "one", "secret"].join("-"),
    });
    const site = useSiteStore.getState().sites.find((item) => item.id === created.id) ?? created;

    render(
      <Wrapper>
        <SiteApiKeyManageModal open site={site} onClose={() => undefined} />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByPlaceholderText("sk-...")).toHaveValue("sk-one-secret");
    });
    fireEvent.click(screen.getByRole("button", { name: "复制" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith("sk-one-secret"));
    expect(await screen.findByText("已复制")).toBeInTheDocument();
  });

  it("tests a key without replacing the current key models", async () => {
    const created = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-one",
    });
    await useSiteStore.getState().addApiKey(created.id, { apiKey: "sk-two", label: "K 2" });
    await useSiteStore.getState().fetchModels(created.id);
    const before = useSiteStore.getState().modelsBySite[created.id] ?? [];
    expect(before.map((model) => model.modelId).sort()).toEqual(["claude-sonnet-4", "gpt-4.1"]);
    const site = useSiteStore.getState().sites.find((item) => item.id === created.id) ?? created;

    render(
      <Wrapper>
        <SiteApiKeyManageModal open site={site} onClose={() => undefined} />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getAllByPlaceholderText("sk-...")).toHaveLength(2);
    });
    fireEvent.click(screen.getAllByRole("button", { name: "测试密钥" })[1]!);
    expect(await screen.findByText("测试成功，获取到 2 个模型")).toBeInTheDocument();
    expect((useSiteStore.getState().modelsBySite[created.id] ?? []).map((model) => model.modelId).sort()).toEqual(
      ["claude-sonnet-4", "gpt-4.1"],
    );
  });

  it("shows the probe error without changing stored models", async () => {
    const created = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-one",
    });
    await useSiteStore.getState().fetchModels(created.id);
    const site = useSiteStore.getState().sites.find((item) => item.id === created.id) ?? created;

    render(
      <Wrapper>
        <SiteApiKeyManageModal open site={site} onClose={() => undefined} />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByPlaceholderText("sk-...")).toHaveValue("sk-one");
    });
    fireEvent.change(screen.getByPlaceholderText("sk-..."), {
      target: { value: "sk-fail" },
    });
    fireEvent.click(screen.getByRole("button", { name: "测试密钥" }));
    expect(await screen.findByText("测试失败：unauthorized")).toBeInTheDocument();
    expect((useSiteStore.getState().modelsBySite[created.id] ?? []).map((model) => model.modelId).sort()).toEqual(
      ["claude-sonnet-4", "gpt-4.1"],
    );
  });

  it("closes after a successful save", async () => {
    // 回归：保存成功只提示、不关闭，用户会以为弹窗卡住了（按钮写的是「保存」）。
    const created = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-one",
    });
    const site = useSiteStore.getState().sites.find((item) => item.id === created.id) ?? created;
    const onClose = vi.fn();

    render(
      <Wrapper>
        <SiteApiKeyManageModal open site={site} onClose={onClose} />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByPlaceholderText("sk-...")).toHaveValue("sk-one");
    });
    fireEvent.change(screen.getByPlaceholderText("sk-..."), {
      target: { value: "sk-updated" },
    });
    fireEvent.click(screen.getByRole("button", { name: /保\s*存/ }));

    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });
});
