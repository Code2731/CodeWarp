import { Channel, invoke } from "@tauri-apps/api/core";

export interface OpenRouterPricing {
  prompt?: string;
  completion?: string;
}

export interface OpenRouterModel {
  id: string;
  name?: string;
  context_length?: number;
  pricing?: OpenRouterPricing;
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
}

export type ChatEvent =
  | { kind: "token"; data: string }
  | { kind: "done" }
  | { kind: "error"; data: string };

export const api = {
  setOpenRouterKey: (key: string) =>
    invoke<void>("set_openrouter_key", { key }),
  hasOpenRouterKey: () => invoke<boolean>("has_openrouter_key"),
  clearOpenRouterKey: () => invoke<void>("clear_openrouter_key"),
  listOpenRouterModels: () =>
    invoke<OpenRouterModel[]>("list_openrouter_models"),

  chatStream(
    model: string,
    messages: ChatMessage[],
    onEvent: (e: ChatEvent) => void,
  ): Promise<void> {
    const channel = new Channel<ChatEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("chat_stream", {
      model,
      messages,
      onEvent: channel,
    });
  },
};
