import { useEffect, useState } from "react";
import { api } from "../lib/api";

interface Props {
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
}

export default function SettingsModal({ open, onClose, onSaved }: Props) {
  const [hasKey, setHasKey] = useState(false);
  const [keyInput, setKeyInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setError(null);
    setInfo(null);
    setKeyInput("");
    api
      .hasOpenRouterKey()
      .then(setHasKey)
      .catch((e) => setError(String(e)));
  }, [open]);

  if (!open) return null;

  async function save() {
    setBusy(true);
    setError(null);
    setInfo(null);
    try {
      await api.setOpenRouterKey(keyInput);
      setHasKey(true);
      setKeyInput("");
      setInfo("저장되었습니다. 모델 리스트를 가져오는 중…");
      onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function clear() {
    setBusy(true);
    setError(null);
    setInfo(null);
    try {
      await api.clearOpenRouterKey();
      setHasKey(false);
      setInfo("API 키가 삭제되었습니다.");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal__header">
          <h2 className="modal__title">Settings</h2>
          <button className="modal__close" onClick={onClose} aria-label="닫기">
            ×
          </button>
        </div>

        <div className="modal__section">
          <div className="modal__label">OpenRouter API Key</div>
          <div className="modal__hint">
            {hasKey
              ? "현재 키가 OS Credential Manager에 저장되어 있습니다."
              : "키가 저장되어 있지 않습니다. 새로 입력해 주세요."}
          </div>
          <input
            type="password"
            className="modal__input"
            value={keyInput}
            onChange={(e) => setKeyInput(e.target.value)}
            placeholder="sk-or-v1-..."
            autoComplete="off"
            spellCheck={false}
            disabled={busy}
          />
          <div className="modal__actions">
            <button
              className="modal__btn modal__btn--primary"
              onClick={save}
              disabled={busy || !keyInput.trim()}
            >
              저장
            </button>
            {hasKey && (
              <button
                className="modal__btn modal__btn--danger"
                onClick={clear}
                disabled={busy}
              >
                삭제
              </button>
            )}
          </div>
          {error && <div className="modal__error">{error}</div>}
          {info && <div className="modal__info">{info}</div>}
        </div>

        <div className="modal__footnote">
          키는 https://openrouter.ai/keys 에서 발급받으세요. 저장된 키는
          평문으로 노출되지 않습니다.
        </div>
      </div>
    </div>
  );
}
