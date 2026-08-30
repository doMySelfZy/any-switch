import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { handleBrowserCommand, resetBrowserMock } from "@/lib/browserMock";
import { useSkillStore } from "@/stores/skillStore";
import type { MarketplaceSkill, Skill } from "@/types/domain";
import { SkillsPage } from "./SkillsPage";
import "@/i18n";

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider theme={{ token: { motion: false } }}>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

function resetSkillStore() {
  useSkillStore.setState({
    skills: [],
    marketplaceSkills: [],
    selectedSkill: null,
    loading: false,
    marketplaceLoading: false,
    hydrated: false,
  });
}

describe("SkillsPage", () => {
  beforeEach(() => {
    resetBrowserMock();
    resetSkillStore();
  });

  afterEach(() => {
    resetBrowserMock();
    resetSkillStore();
  });

  it("filters installed skills by target", async () => {
    render(
      <Wrapper>
        <SkillsPage />
      </Wrapper>,
    );

    expect(await screen.findByText("Shared workflows installed for Claude Code")).toBeInTheDocument();
    expect(screen.getByText("Shared agent skills installed under ~/.agents/skills")).toBeInTheDocument();
    expect(screen.getByTestId("skill-list")).toHaveStyle({ overflowY: "auto" });
    expect(screen.getByText("The same skill installed independently for Codex")).toBeInTheDocument();

    const tabLabels = screen.getAllByRole("tab").map((tab) => tab.textContent ?? "");
    const allIndex = tabLabels.findIndex((label) => /全部|All/.test(label));
    const agentsIndex = tabLabels.findIndex((label) => label.includes("Agents"));
    expect(allIndex).toBeGreaterThanOrEqual(0);
    expect(agentsIndex).toBe(allIndex + 1);

    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));

    await waitFor(() => {
      expect(screen.queryByText("Shared workflows installed for Claude Code")).toBeNull();
    });
    expect(screen.getByText("The same skill installed independently for Codex")).toBeInTheDocument();
    expect(screen.getByText("/Users/demo/.codex/skills")).toBeInTheDocument();
  });

  it("filters the shared ~/.agents/skills target after All", async () => {
    render(
      <Wrapper>
        <SkillsPage />
      </Wrapper>,
    );

    await screen.findByText("Shared agent skills installed under ~/.agents/skills");
    fireEvent.click(screen.getByRole("tab", { name: "Agents" }));

    await waitFor(() => {
      expect(screen.queryByText("Shared workflows installed for Claude Code")).toBeNull();
    });
    expect(screen.getByText("Shared agent skills installed under ~/.agents/skills")).toBeInTheDocument();
    expect(screen.getByText("/Users/demo/.agents/skills")).toBeInTheDocument();
  });

  it("toggles only the selected target and source path when names collide", async () => {
    render(
      <Wrapper>
        <SkillsPage />
      </Wrapper>,
    );

    await screen.findByText("The same skill installed independently for Codex");
    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
    fireEvent.click(screen.getByRole("switch"));

    await waitFor(async () => {
      const skills = await handleBrowserCommand<Skill[]>("list_skills");
      const claude = skills.find(
        (skill) => skill.target === "claude_code" && skill.name === "shared-tools",
      );
      const codex = skills.find(
        (skill) => skill.target === "codex" && skill.name === "shared-tools",
      );
      expect(claude?.enabled).toBe(true);
      expect(codex?.enabled).toBe(false);
      expect(codex?.sourcePath).toBe(
        "/Users/demo/.codex/skills/shared-tools/SKILL.md.disabled",
      );
    });

    await waitFor(() => expect(screen.getByRole("switch")).not.toBeChecked());
    fireEvent.click(screen.getByRole("switch"));

    await waitFor(async () => {
      const skills = await handleBrowserCommand<Skill[]>("list_skills");
      const codex = skills.find(
        (skill) => skill.target === "codex" && skill.name === "shared-tools",
      );
      expect(codex?.enabled).toBe(true);
      expect(codex?.sourcePath).toBe(
        "/Users/demo/.codex/skills/shared-tools/SKILL.md",
      );
    });
  });

  it("passes the selected install target to the browser command", async () => {
    render(
      <Wrapper>
        <SkillsPage />
      </Wrapper>,
    );

    await screen.findByText("Shared workflows installed for Claude Code");
    fireEvent.change(screen.getByPlaceholderText(/owner\/repo|GitHub URL/i), {
      target: { value: "demo/design-system" },
    });
    fireEvent.click(screen.getByRole("button", { name: /URL/ }));
    const installTargets = await screen.findAllByRole("menuitem");
    expect(installTargets[0]).toHaveTextContent("Agents");
    fireEvent.click(await screen.findByRole("menuitem", { name: "Pi" }));

    await waitFor(async () => {
      const skills = await handleBrowserCommand<Skill[]>("list_skills");
      expect(skills).toContainEqual(
        expect.objectContaining({
          name: "design-system",
          target: "pi",
          sourcePath: "/Users/demo/.pi/agent/skills/design-system/SKILL.md",
        }),
      );
    });
  });

  it("loads marketplace results when opening the marketplace tab", async () => {
    render(
      <Wrapper>
        <SkillsPage />
      </Wrapper>,
    );

    await screen.findByText("Shared workflows installed for Claude Code");
    fireEvent.click(screen.getByRole("tab", { name: /在线市场|Marketplace/ }));

    expect(await screen.findByText("Reusable coding-agent workflows")).toBeInTheDocument();
    expect(screen.getByTestId("marketplace-list")).toHaveStyle({ overflowY: "auto" });
    expect(screen.getByText("demo/shared-tools")).toBeInTheDocument();
  });

  it("keeps platform filter tabs as a single accessible tablist", async () => {
    render(
      <Wrapper>
        <SkillsPage />
      </Wrapper>,
    );

    await screen.findByText("Shared workflows installed for Claude Code");
    const tabs = screen.getAllByRole("tab", { name: "Claude Code" });
    expect(tabs).toHaveLength(1);
    expect(tabs[0].textContent).toContain("Claude Code");
  });

  it("models marketplace sources and tracks installed targets by source ref", async () => {
    const skillsSh = await handleBrowserCommand<MarketplaceSkill[]>(
      "search_skill_marketplace",
      { query: "design", source: "skills.sh" },
    );
    expect(skillsSh[0]).toEqual(expect.objectContaining({ stars: 0, installs: 1024 }));

    const github = await handleBrowserCommand<MarketplaceSkill[]>(
      "search_skill_marketplace",
      { query: "design", source: "github" },
    );
    expect(github[0]).toEqual(expect.objectContaining({ stars: 96, installs: 0 }));

    await handleBrowserCommand("install_skill", {
      source: "https://github.com/Demo/Design-System.git",
      target: "pi",
    });
    const installed = await handleBrowserCommand<MarketplaceSkill[]>(
      "search_skill_marketplace",
      { query: "design", source: "github" },
    );
    expect(installed[0]?.installedTargets).toEqual(["pi"]);

    await handleBrowserCommand("set_skill_enabled", {
      target: "pi",
      sourcePath: "/Users/demo/.pi/agent/skills/design-system/SKILL.md",
      enabled: false,
    });
    await handleBrowserCommand("uninstall_skill", {
      target: "pi",
      sourcePath: "/Users/demo/.pi/agent/skills/design-system/SKILL.md.disabled",
    });
    const uninstalled = await handleBrowserCommand<MarketplaceSkill[]>(
      "search_skill_marketplace",
      { query: "design", source: "github" },
    );
    expect(uninstalled[0]?.installedTargets).toEqual([]);
  });
});
