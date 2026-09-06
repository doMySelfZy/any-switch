import { fireEvent, render, screen } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import { describe, expect, it, vi } from "vitest";
import { emptyThinkingPreset } from "@/lib/thinkingPreset";
import type { SiteModel } from "@/types/domain";
import { ThinkingPresetEditor } from "./ThinkingPresetEditor";
import "@/i18n";

const models: SiteModel[] = [
  {
    id: "row-a",
    siteId: "s1",
    modelId: "model-a",
    displayName: "Model A",
    ownedBy: null,
    raw: null,
  },
];

describe("ThinkingPresetEditor", () => {
  it("enables reasoning without choosing a default level", () => {
    const onChange = vi.fn();
    render(
      <ConfigProvider>
        <AntdApp>
          <ThinkingPresetEditor
            preset={emptyThinkingPreset("s1", "pi")}
            protocol="openai_compatible"
            models={models}
            defaultModelId="model-a"
            onChange={onChange}
          />
        </AntdApp>
      </ConfigProvider>,
    );
    fireEvent.click(screen.getAllByRole("switch")[0]);
    expect(onChange).toHaveBeenCalled();
    const next = onChange.mock.calls[0][0];
    expect(next.models["model-a"].reasoning).toBe(true);
    expect(next.defaultLevel ?? null).toBeNull();
  });
});
