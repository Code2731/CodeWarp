import { useEffect, useRef, useState } from "react";
import { api, type ChatMessage } from "../lib/api";

type Role = "user" | "assistant";

interface Block {
  id: string;
  role: Role;
  content: string;
  pending?: boolean;
  errored?: boolean;
}

interface Props {
  selectedModel: string | null;
}

export default function BlockTerminal({ selectedModel }: Props) {
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [input, setInput] = useState("");
  const [streaming, setStreaming] = useState(false);
  const streamRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    streamRef.current?.scrollTo({
      top: streamRef.current.scrollHeight,
      behavior: "smooth",
    });
  }, [blocks]);

  async function send() {
    const text = input.trim();
    if (!text || streaming) return;
    if (!selectedModel) {
      alert("모델이 선택되지 않았습니다. Settings에서 OpenRouter 키를 등록하세요.");
      return;
    }

    const userBlock: Block = {
      id: crypto.randomUUID(),
      role: "user",
      content: text,
    };
    const aiBlock: Block = {
      id: crypto.randomUUID(),
      role: "assistant",
      content: "",
      pending: true,
    };

    // 스냅샷 기반 history (현재 blocks + 새 user). 빈 assistant는 미포함.
    const history: ChatMessage[] = [
      ...blocks
        .filter((b) => b.content.trim() && !b.errored)
        .map((b) => ({ role: b.role, content: b.content })),
      { role: "user", content: text },
    ];

    setBlocks((b) => [...b, userBlock, aiBlock]);
    setInput("");
    setStreaming(true);

    try {
      await api.chatStream(selectedModel, history, (evt) => {
        if (evt.kind === "token") {
          setBlocks((bs) =>
            bs.map((b) =>
              b.id === aiBlock.id ? { ...b, content: b.content + evt.data } : b,
            ),
          );
        } else if (evt.kind === "done") {
          setBlocks((bs) =>
            bs.map((b) => (b.id === aiBlock.id ? { ...b, pending: false } : b)),
          );
        } else if (evt.kind === "error") {
          setBlocks((bs) =>
            bs.map((b) =>
              b.id === aiBlock.id
                ? {
                    ...b,
                    content: b.content
                      ? `${b.content}\n\n[에러] ${evt.data}`
                      : `[에러] ${evt.data}`,
                    pending: false,
                    errored: true,
                  }
                : b,
            ),
          );
        }
      });
    } catch (e) {
      setBlocks((bs) =>
        bs.map((b) =>
          b.id === aiBlock.id
            ? {
                ...b,
                content: `[에러] ${e}`,
                pending: false,
                errored: true,
              }
            : b,
        ),
      );
    } finally {
      setStreaming(false);
    }
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  return (
    <section className="terminal">
      <div className="terminal__stream" ref={streamRef}>
        {blocks.length === 0 ? (
          <div className="terminal__empty">
            $ CodeWarp ready — 모델 선택 후 입력하세요
          </div>
        ) : (
          blocks.map((b) => (
            <div
              key={b.id}
              className={`block block--${b.role} ${
                b.errored ? "block--errored" : ""
              }`}
            >
              <div className="block__header">
                <span className="block__role-dot" />
                <span>{b.role === "user" ? "you" : "ai"}</span>
                {b.pending && (
                  <span className="block__status">스트리밍…</span>
                )}
              </div>
              <div className="block__body">
                {b.content || (b.pending ? "…" : "")}
              </div>
            </div>
          ))
        )}
      </div>
      <div className="terminal__input">
        <textarea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={
            streaming
              ? "응답 생성 중…"
              : "명령이나 질문을 입력하세요…  (Enter = 전송, Shift+Enter = 줄바꿈)"
          }
          rows={1}
          disabled={streaming}
        />
        <button
          className="terminal__send"
          onClick={send}
          disabled={streaming || !input.trim() || !selectedModel}
        >
          {streaming ? "…" : "Send"}
        </button>
      </div>
    </section>
  );
}
