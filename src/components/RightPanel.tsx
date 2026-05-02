import { useState } from "react";

type Tab = "plan" | "diff" | "history";

export default function RightPanel() {
  const [tab, setTab] = useState<Tab>("plan");

  return (
    <aside className="rightpanel">
      <div className="rightpanel__tabs">
        {(["plan", "diff", "history"] as Tab[]).map((t) => (
          <button
            key={t}
            className={`rightpanel__tab ${
              tab === t ? "rightpanel__tab--active" : ""
            }`}
            onClick={() => setTab(t)}
          >
            {t}
          </button>
        ))}
      </div>
      <div className="rightpanel__placeholder">
        {tab === "plan" && "// Plan 모드: 에이전트 단계가 여기 표시됩니다."}
        {tab === "diff" && "// Diff Preview: 변경된 파일이 여기 표시됩니다."}
        {tab === "history" && "// 채팅 / 세션 이력이 여기 표시됩니다."}
      </div>
    </aside>
  );
}
