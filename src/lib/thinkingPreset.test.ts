import { describe, expect, it } from "vitest";
import {
  allowsExtendedLevels,
  emptyThinkingPreset,
  thinkingValidationKey,
} from "./thinkingPreset";

describe("thinkingPreset", () => {
  it("allows empty presets", () => {
    const preset = emptyThinkingPreset("s1", "pi");
    expect(thinkingValidationKey(preset, "openai_compatible", "model-a")).toBeNull();
  });

  it("requires reasoning on the default model", () => {
    const preset = emptyThinkingPreset("s1", "pi");
    preset.defaultLevel = "medium";
    expect(thinkingValidationKey(preset, "openai_compatible", "model-a")).toBe(
      "apply.thinkingNeedReasoning",
    );
  });

  it("requires an extended mapping", () => {
    const preset = emptyThinkingPreset("s1", "pi");
    preset.defaultLevel = "max";
    preset.models["model-a"] = { reasoning: true };
    expect(thinkingValidationKey(preset, "openai_compatible", "model-a")).toBe(
      "apply.thinkingNeedMapping",
    );
    preset.extended.max = "max";
    expect(thinkingValidationKey(preset, "openai_compatible", "model-a")).toBeNull();
  });

  it("blocks Prime Anthropic extended levels", () => {
    const preset = emptyThinkingPreset("s1", "prime");
    preset.defaultLevel = "xhigh";
    preset.extended.xhigh = "xhigh";
    preset.models["model-a"] = { reasoning: true };
    expect(thinkingValidationKey(preset, "anthropic", "model-a")).toBe(
      "apply.thinkingPrimeAnthropicNoExtended",
    );
    expect(allowsExtendedLevels("prime", "anthropic", true)).toBe(false);
  });

  it("requires Pi Anthropic adaptive for extended levels", () => {
    const preset = emptyThinkingPreset("s1", "pi");
    preset.defaultLevel = "xhigh";
    preset.extended.xhigh = "xhigh";
    preset.models["model-a"] = { reasoning: true };
    expect(thinkingValidationKey(preset, "anthropic", "model-a")).toBe(
      "apply.thinkingNeedAdaptive",
    );
    preset.models["model-a"].forceAdaptiveThinking = true;
    expect(thinkingValidationKey(preset, "anthropic", "model-a")).toBeNull();
  });
});
