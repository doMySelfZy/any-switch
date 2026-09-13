import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { handleBrowserCommand, resetBrowserMock } from "@/lib/browserMock";
import { resetRulesStore } from "@/stores/rulesStore";
import { RulesPage } from "./RulesPage";
import "@/i18n";

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider theme={{ token: { motion: false } }}>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

/** antd 会在两个汉字之间插入空格，这里统一用容忍空白的匹配。 */
function saveButton(): HTMLElement {
  return screen.getByRole("button", { name: /保\s*存/ });
}

function bodyInput(): HTMLElement {
  return screen.getByLabelText("约束正文");
}

function targetCheckbox(name: string): HTMLElement {
  return screen.getByRole("checkbox", { name });
}

describe("RulesPage", () => {
  beforeEach(() => {
    resetBrowserMock();
    resetRulesStore();
  });

  afterEach(() => {
    resetBrowserMock();
    resetRulesStore();
  });

  it("shows the four target file locations", async () => {
    render(
      <Wrapper>
        <RulesPage />
      </Wrapper>,
    );

    // 四个目标的落点都要能看到，用户才知道会写到哪里。
    expect(await screen.findByText("/Users/demo/.claude/CLAUDE.md")).toBeInTheDocument();
    expect(screen.getByText("/Users/demo/.codex/AGENTS.md")).toBeInTheDocument();
    expect(screen.getByText("/Users/demo/.pi/agent/AGENTS.md")).toBeInTheDocument();
    expect(screen.getByText("/Users/demo/.prime/agent/AGENTS.md")).toBeInTheDocument();
  });

  it("saves the body to the selected targets and reports each result", async () => {
    render(
      <Wrapper>
        <RulesPage />
      </Wrapper>,
    );

    fireEvent.change(await screen.findByLabelText("约束正文"), {
      target: { value: "# 全局约束\n- 说中文" },
    });
    fireEvent.click(targetCheckbox("Claude Code"));
    fireEvent.click(targetCheckbox("Codex"));
    fireEvent.click(saveButton());

    // 逐目标回报写入结果，而不是一个笼统的成功提示。
    expect(await screen.findByText(/写入结果/)).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getAllByText(/已写入/).length).toBe(2);
    });

    const stored = await handleBrowserCommand("get_agent_rules", {});
    expect(stored).toMatchObject({
      body: "# 全局约束\n- 说中文",
      targets: ["claude_code", "codex"],
    });
  });

  it("explains that an empty body removes what the app wrote", async () => {
    await handleBrowserCommand("save_agent_rules", {
      body: "# 之前写的",
      targets: ["claude_code"],
    });

    render(
      <Wrapper>
        <RulesPage />
      </Wrapper>,
    );

    // 载入库里那份内容后，清空正文应提示「会移除托管内容」而不是写空块。
    await waitFor(() => {
      expect((bodyInput() as HTMLTextAreaElement).value).toBe("# 之前写的");
    });
    fireEvent.change(bodyInput(), { target: { value: "" } });

    expect(await screen.findByText(/会移除各文件里由本应用写入的内容/)).toBeInTheDocument();
  });

  it("rejects rules text containing the managed-block markers", async () => {
    render(
      <Wrapper>
        <RulesPage />
      </Wrapper>,
    );

    fireEvent.change(await screen.findByLabelText("约束正文"), {
      target: { value: "<!-- xiaobai-switch:begin global-rules -->" },
    });
    fireEvent.click(targetCheckbox("Pi"));
    fireEvent.click(saveButton());

    expect(await screen.findByText(/保存失败/)).toBeInTheDocument();
  });
});
