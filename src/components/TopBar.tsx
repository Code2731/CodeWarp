import type { OpenRouterModel } from "../lib/api";

interface Props {
  models: OpenRouterModel[];
  selectedModel: string | null;
  onSelectModel: (id: string) => void;
  onOpenSettings: () => void;
  loading: boolean;
}

export default function TopBar({
  models,
  selectedModel,
  onSelectModel,
  onOpenSettings,
  loading,
}: Props) {
  return (
    <header className="topbar">
      <div className="topbar__brand">
        <span className="topbar__brand-mark" />
        <span>CodeWarp</span>
      </div>
      <div className="topbar__spacer" />
      {loading ? (
        <div className="topbar__model-select" aria-busy>
          모델 불러오는 중…
        </div>
      ) : models.length === 0 ? (
        <div
          className="topbar__model-select"
          title="Settings에서 OpenRouter API 키를 등록하세요"
        >
          모델 없음
        </div>
      ) : (
        <select
          className="topbar__model-select"
          value={selectedModel ?? ""}
          onChange={(e) => onSelectModel(e.target.value)}
        >
          {models.map((m) => (
            <option key={m.id} value={m.id}>
              {m.id}
            </option>
          ))}
        </select>
      )}
      <button
        className="topbar__icon-btn"
        onClick={onOpenSettings}
        aria-label="Settings"
        title="Settings"
      >
        ⚙
      </button>
    </header>
  );
}
