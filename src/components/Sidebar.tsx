export default function Sidebar() {
  return (
    <aside className="sidebar">
      <div className="sidebar__section-title">프로젝트</div>
      <div className="sidebar__item sidebar__item--active">CodeWarp</div>
      <div className="sidebar__section-title" style={{ marginTop: 14 }}>
        파일
      </div>
      <div className="sidebar__item">src/</div>
      <div className="sidebar__item">src-tauri/</div>
      <div className="sidebar__item">README.md</div>
      <div className="sidebar__section-title" style={{ marginTop: 14 }}>
        컨텍스트
      </div>
      <div className="sidebar__item">선택 안 됨</div>
    </aside>
  );
}
