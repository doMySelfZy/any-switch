import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { handleBrowserCommand, resetBrowserMock } from "@/lib/browserMock";
import { useMcpStore } from "@/stores/mcpStore";
import { McpPage } from "./McpPage";
import "@/i18n";

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider theme={{ token: { motion: false } }}>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

function resetMcpStore() {
  useMcpStore.setState({ servers: [], loading: false });
}

/** antd 会在两个汉字之间插入空格，这里统一用容忍空白的匹配。 */
function saveButton(): HTMLElement {
  return screen.getByRole("button", { name: /保\s*存/ });
}

async function seedServer(overrides: Record<string, unknown> = {}) {
  return handleBrowserCommand("save_mcp_server", {
    input: {
      name: "demo",
      kind: "stdio",
      enabled: true,
      targets: ["claude_code"],
      config: { command: "npx" },
      env: {},
      headers: {},
      ...overrides,
    },
  });
}

describe("McpPage", () => {
  beforeEach(() => {
    resetBrowserMock();
    resetMcpStore();
  });

  afterEach(() => {
    resetBrowserMock();
    resetMcpStore();
  });

  it("shows the empty state and explains where configs are written", async () => {
    render(
      <Wrapper>
        <McpPage />
      </Wrapper>,
    );

    expect(await screen.findByText("还没有 MCP 服务")).toBeInTheDocument();
    expect(screen.getByText(/环境变量与 Headers 在本应用内加密保存/)).toBeInTheDocument();
    expect(screen.getByText("/Users/demo/.claude.json")).toBeInTheDocument();
    expect(screen.getByText("/Users/demo/.prime/agent/settings.json")).toBeInTheDocument();
  });

  it("lists saved servers with their targets", async () => {
    await seedServer({ targets: ["claude_code", "prime"] });

    render(
      <Wrapper>
        <McpPage />
      </Wrapper>,
    );

    expect(await screen.findByText("demo")).toBeInTheDocument();
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("Prime")).toBeInTheDocument();
  });

  it("rejects a server name with characters that would break config keys", async () => {
    render(
      <Wrapper>
        <McpPage />
      </Wrapper>,
    );

    fireEvent.click(await screen.findByRole("button", { name: /添加 MCP 服务/ }));
    const nameInput = await screen.findByLabelText("服务名称");
    fireEvent.change(nameInput, { target: { value: "bad name" } });
    fireEvent.click(saveButton());

    await waitFor(() => {
      expect(screen.getByText("只能使用字母、数字、下划线和短横线")).toBeInTheDocument();
    });
  });

  it("reports invalid JSON instead of saving it", async () => {
    render(
      <Wrapper>
        <McpPage />
      </Wrapper>,
    );

    fireEvent.click(await screen.findByRole("button", { name: /添加 MCP 服务/ }));
    fireEvent.change(await screen.findByLabelText("服务名称"), {
      target: { value: "filesystem" },
    });
    const configInput = screen.getByLabelText("配置");
    fireEvent.change(configInput, { target: { value: "{not json" } });
    fireEvent.click(saveButton());

    await waitFor(() => {
      expect(screen.getByText(/不是合法的 JSON/)).toBeInTheDocument();
    });
    expect(useMcpStore.getState().servers).toHaveLength(0);
  });

  it("applies only the targets that enabled servers point at", async () => {
    await seedServer({ targets: ["claude_code"] });
    await seedServer({ name: "remote", targets: ["codex"] });

    render(
      <Wrapper>
        <McpPage />
      </Wrapper>,
    );

    fireEvent.click(await screen.findByRole("button", { name: /应用到目标/ }));

    // 两个目标都有启用的服务指向它们，应用结果逐目标列出。
    expect(await screen.findByText(/Claude Code 应用成功/)).toBeInTheDocument();
    expect(screen.getByText(/Codex 应用成功/)).toBeInTheDocument();
  });

  it("disables apply when no enabled server selects a target", async () => {
    await seedServer({ enabled: false, targets: ["claude_code"] });

    render(
      <Wrapper>
        <McpPage />
      </Wrapper>,
    );

    const applyButton = await screen.findByRole("button", { name: /应用到目标/ });
    await waitFor(() => {
      expect(applyButton).toBeDisabled();
    });
  });

  it("round-trips an edited server back into the form", async () => {
    const saved = (await seedServer({
      targets: ["codex"],
      config: { command: "npx", args: ["-y", "pkg"] },
      env: { TOKEN: "abc" },
    })) as { server: { id: string } };

    render(
      <Wrapper>
        <McpPage />
      </Wrapper>,
    );

    const row = (await screen.findByText("demo")).closest("tr")!;
    fireEvent.click(within(row).getByRole("button", { name: /编\s*辑/ }));

    const nameInput = (await screen.findByLabelText("服务名称")) as HTMLInputElement;
    await waitFor(() => {
      expect(nameInput.value).toBe("demo");
    });
    const configInput = screen.getByLabelText("配置") as HTMLTextAreaElement;
    expect(configInput.value).toContain("npx");
    // 已保存的密钥回填到编辑框，用户不必重新输入。
    expect((screen.getByLabelText("环境变量") as HTMLTextAreaElement).value).toContain("TOKEN");
    expect(saved.server.id).toBeTruthy();
  });

  it("reports per-target results in the apply dialog", async () => {
    await seedServer({ targets: ["claude_code"] });

    render(
      <Wrapper>
        <McpPage />
      </Wrapper>,
    );

    fireEvent.click(await screen.findByRole("button", { name: /应用到目标/ }));

    // 应用结果弹窗对目标逐一给出结果。
    const dialogs = await screen.findAllByRole("dialog");
    const text = dialogs.map((dialog) => dialog.textContent ?? "").join("\n");
    expect(text).toContain("应用结果");
    expect(text).toMatch(/Claude Code 应用成功/);
  });
});
