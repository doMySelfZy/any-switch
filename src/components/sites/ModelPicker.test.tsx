import { render, screen } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useSiteStore } from "@/stores";
import type { Site } from "@/types/domain";
import { ModelPicker } from "./ModelPicker";
import i18n from "@/i18n";

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

function site(lastModelFetchError: string): Site {
  return {
    id: "site-1",
    name: "Relay",
    baseUrl: "https://api.example.com",
    baseUrls: ["https://api.example.com"],
    keyPrefix: "sk-t…",
    quotaRevision: "rev-1",
    hasKey: true,
    protocol: "openai_compatible",
    claudeAuthKeyStyle: "anthropic_auth_token",
    notes: null,
    enabled: true,
    sortOrder: 0,
    selectedModelId: null,
    lastModelFetchAt: null,
    lastModelFetchLatencyMs: null,
    lastModelFetchError,
    createdAt: 1,
    updatedAt: 1,
  };
}

describe("ModelPicker fetch errors", () => {
  beforeEach(() => {
    useSiteStore.setState({ modelsBySite: {}, fetchingModels: false });
  });

  afterEach(async () => {
    await i18n.changeLanguage("zh-CN");
  });

  it("maps a persisted unauthorized error to actionable localized copy", () => {
    render(
      <Wrapper>
        <ModelPicker site={site("unauthorized")} models={[]} />
      </Wrapper>,
    );

    expect(screen.getByText("模型列表获取失败")).toBeInTheDocument();
    expect(screen.getByText("鉴权失败，请检查 API Key 或站点协议")).toBeInTheDocument();
    expect(screen.queryByText("unauthorized")).toBeNull();
    expect(screen.getByRole("alert")).toBeInTheDocument();
  });

  it("only exposes a sanitized HTTP status as technical details", () => {
    render(
      <Wrapper>
        <ModelPicker site={site("HTTP 502 from upstream")} models={[]} />
      </Wrapper>,
    );

    expect(screen.getByText("模型列表获取失败")).toBeInTheDocument();
    expect(screen.getByText("技术详情：HTTP 502")).toBeInTheDocument();
    expect(screen.queryByText(/from upstream/)).toBeNull();
  });

  it("does not render an unknown persisted error that may contain a secret", () => {
    render(
      <Wrapper>
        <ModelPicker site={site("proxy failed for sk-super-secret")} models={[]} />
      </Wrapper>,
    );

    expect(screen.getByText("请重试或检查站点配置")).toBeInTheDocument();
    expect(screen.queryByText(/sk-super-secret/)).toBeNull();
  });

  it("renders the actionable model error in English", async () => {
    await i18n.changeLanguage("en-US");
    render(
      <Wrapper>
        <ModelPicker site={site("unauthorized")} models={[]} />
      </Wrapper>,
    );

    expect(screen.getByText("Could not fetch the model list")).toBeInTheDocument();
    expect(
      screen.getByText("Authentication failed; check the API key or site protocol"),
    ).toBeInTheDocument();
  });
});
