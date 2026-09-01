import { isTauri } from "@/lib/invoke";

export async function copyText(text: string): Promise<void> {
  if (isTauri()) {
    const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
    await writeText(text);
    return;
  }
  if (!navigator.clipboard?.writeText) {
    throw { code: "internal", message: "clipboard unavailable" };
  }
  await navigator.clipboard.writeText(text);
}
