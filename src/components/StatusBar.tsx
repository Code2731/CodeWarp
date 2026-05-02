interface Props {
  hasKey: boolean;
  modelCount: number;
  message: string;
}

export default function StatusBar({ hasKey, modelCount, message }: Props) {
  return (
    <footer className="statusbar">
      <span
        className={`statusbar__dot ${hasKey ? "" : "statusbar__dot--off"}`}
      />
      <span>{message}</span>
      <span className="statusbar__spacer" />
      <span>
        OpenRouter:{" "}
        {hasKey ? `연결됨 (${modelCount} models)` : "키 미등록"}
      </span>
      <span>·</span>
      <span>v0.1.0</span>
    </footer>
  );
}
