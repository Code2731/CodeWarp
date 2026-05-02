# CodeWarp

Warp 스타일의 AI 코딩 데스크톱. **Iced (Rust 네이티브 GUI)** 기반.

> 개발자가 진짜 쓰고 싶을 만큼 정교하고, 어떤 LLM이든 자유롭게 쓸 수 있는 AI 코딩 에이전트.

---

## 현재 상태 — Phase 2 (2026-05)

- [x] **Phase 0** — Tauri 2.0 + React 19로 OpenRouter 단일턴 채팅까지 (커밋 이력에 보존)
- [x] **Phase 1A** — Markdown 렌더링 시도 + Tauri webview의 scroll/overflow 한계 확인
- [x] **Phase 2-1** — Iced 0.14 베이스 부팅 + Pretendard 폰트 임베드 (한국어 표시)
- [x] **Phase 2-2** — keyring(OS Credential Manager) + OpenRouter HTTP 포팅
- [x] **Phase 2-3** — 3-pane 레이아웃 + 모델 셀렉터 + 채팅 1턴 SSE 스트리밍
- [x] **Phase 2-3 추가** — 블록 단위 복사 버튼, 좁은 스크롤바, IME 입력 지원
- [ ] **Phase 2-4** — 자동 스크롤, 부분 텍스트 선택 복사, Markdown + syntax highlight
- [ ] Phase 2-5 — 도구 호출 루프 (read/write/glob/grep)
- [ ] Phase 2-6 — Plan ↔ Build 모드, 명령 팔레트, PTY 터미널

## 왜 Iced로 바꿨나

Tauri(webview2/Edge) 환경에서 `::-webkit-scrollbar`, `flex + overflow` 조합이 일관성 없게 동작하는 알려진 케이스 ([tauri#8829](https://github.com/tauri-apps/tauri/discussions/8829), [tauri#5501](https://github.com/tauri-apps/tauri/discussions/5501))가 있어, SDD에 적어둔 **2단계 ("Rust Native (Iced 또는 egui) → Warp급 성능")** 로 직진.

Warp 본체도 2026년 4월 오픈소스화되어 ([warpdotdev/Warp](https://github.com/warpdotdev/Warp)) 자체 GPU UI 프레임워크 `warpui`(MIT)를 공개했다. 검증되면 차후 도입 검토.

## 아키텍처

```
┌──────────────────────────────────────────────┐
│  iced::application (Elm-architecture)         │
│  ─ State (App)                                │
│  ─ Message (UI 이벤트, 비동기 결과, 토큰 청크) │
│  ─ update(state, msg) -> Task                 │
│  ─ view(state) -> Element                     │
│                                               │
│  Subsystems                                   │
│  ─ keystore.rs   : keyring (OS Credential)    │
│  ─ openrouter.rs : reqwest + SSE 스트림       │
│  ─ tokio         : async runtime              │
│  ─ async-stream  : Stream<ChatEvent>          │
└──────────────────────────────────────────────┘
```

Tauri/webview/JS 의존성 일체 없음. 단일 Cargo 프로젝트.

## 빠른 시작

### 사전 요구사항

- Rust 1.80+ (`rustup`)
- Windows / macOS / Linux 데스크톱
- OpenRouter API 키 — <https://openrouter.ai/keys>

### 개발 모드

```bash
git clone https://github.com/Code2731/CodeWarp.git
cd CodeWarp
cargo run
```

처음 실행하면 Settings 화면이 뜹니다. OpenRouter 키를 입력하면 OS Credential Manager(Windows) / Keychain(macOS) / Secret Service(Linux)에 저장되며, 모델 리스트가 자동으로 페치됩니다.

### 릴리스 빌드

```bash
cargo build --release
```

`target/release/codewarp(.exe)` 생성.

## 보안

- **API 키는 평문으로 디스크에 저장하지 않습니다.** OS의 Credential Manager에 위임합니다.
- 키 자체는 코드/로그/git 어디에도 출력되지 않습니다 (저장 시 길이만 로그 가능).

## 폰트

[Pretendard](https://github.com/orioncactus/pretendard) (Regular weight)를 binary에 임베드합니다. **SIL Open Font License 1.1** 라이선스이며, `assets/fonts/LICENSE.txt`에 전체 라이선스가 포함되어 있습니다.

## 라이선스

- 본 코드: MIT OR Apache-2.0
- Pretendard 폰트: SIL Open Font License 1.1 (`assets/fonts/LICENSE.txt`)
