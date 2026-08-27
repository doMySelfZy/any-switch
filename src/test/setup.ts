import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";
import { createElement } from "react";

vi.mock("@lobehub/icons/es/ClaudeCode", () => ({
  default: (props: { size?: number }) =>
    createElement("span", { "data-icon": "claude-code", ...props }),
}));

vi.mock("@lobehub/icons/es/Codex", () => ({
  default: (props: { size?: number }) =>
    createElement("span", { "data-icon": "codex", ...props }),
}));

vi.mock("@lobehub/icons/es/Pi", () => ({
  default: (props: { size?: number }) => createElement("span", { "data-icon": "pi", ...props }),
}));

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => undefined,
    removeListener: () => undefined,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
    dispatchEvent: () => false,
  }),
});

class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}

window.ResizeObserver = ResizeObserverMock;

class ImageMock {
  onload: ((ev?: unknown) => void) | null = null;
  onerror: ((ev?: unknown) => void) | null = null;
  set src(_url: string) {
    queueMicrotask(() => this.onerror?.(new Event("error")));
  }
}

window.Image = ImageMock as unknown as typeof Image;
