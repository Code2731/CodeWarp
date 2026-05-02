# CodeWarp

Warp 스타일의 아름다운 터미널 GUI · 자체 구현 AI 코딩 에이전트 · OpenRouter를 통한 모델 완전 자유.

> 개발자가 진짜 쓰고 싶을 만큼 예쁘고, 접근성이 좋으며, 어떤 LLM이든 자유롭게 쓸 수 있는 차세대 AI 코딩 데스크톱 앱.

---

## 현재 상태 — Phase 0 (2026-05)

스캐폴딩 + 핵심 흐름 동작 확인까지 완료.

- [x] Tauri 2.0 + React 19 + TypeScript 베이스
- [x] 다크 테마 3단 레이아웃 (좌: 컨텍스트 / 중: 블록 터미널 / 우: Plan·Diff·History)
- [x] OpenRouter API 키 OS Credential Manager 저장 (`keyring`)
- [x] 모델 리스트 페치 + 셀렉터
- [x] 단일턴 채팅 스트리밍 (Tauri `Channel` + OpenRouter SSE)

다음 단계는 [로드맵](#로드맵) 참고.

---

## 핵심 지향점

1. **시각적 폴리시** — Warp 수준의 블록 기반 UI, 부드러운 애니메이션, 타이포그래피·여백·다크 톤 모두 정교하게.
2. **모델 자유도** — OpenRouter로 Claude / GPT / Gemini / Qwen / Llama 등 어떤 모델이든. 로컬 Ollama 자동 감지(예정).
3. **진짜 코딩 에이전트** — 파일 IO · 검색 · 리팩토링 · LSP 인지를 자체 구현. 외부 코어 의존 없음.
4. **접근성 우선** — 키보드 네비게이션, 스크린리더, 고대비/색맹 모드 (WCAG 2.1 AA 목표).

---

## 빠른 시작

### 사전 요구사항

- Node.js 20+
- Rust 1.80+ (`rustup`으로 stable)
- 플랫폼별 Tauri 의존성: <https://v2.tauri.app/start/prerequisites/>
- OpenRouter API 키 — <https://openrouter.ai/keys>

### 개발 모드 실행

```bash
git clone https://github.com/Code2731/CodeWarp.git
cd CodeWarp
npm install
npm run tauri dev
```

첫 실행 시 우측 상단 ⚙ 클릭 → Settings 모달에 API 키 입력 → 모델 셀렉터에서 모델 선택 → 입력창에 메시지 후 Enter.

### 프로덕션 빌드

```bash
npm run tauri build
```

`src-tauri/target/release/bundle/` 에 OS별 인스톨러가 생성됩니다.

---

## 아키텍처

```
┌────────────────────────────────────────────────┐
│  Frontend (React 19 + Vite)                     │
│  ─ src/App.tsx, src/components/*, src/lib/api.ts│
└────────────────┬───────────────────────────────┘
                 │  Tauri invoke / Channel
                 ▼
┌────────────────────────────────────────────────┐
│  Backend (Rust, src-tauri/src/lib.rs)           │
│  ─ keyring        : OS Credential Manager       │
│  ─ reqwest        : OpenRouter HTTP/SSE         │
│  ─ Channel<Event> : 토큰 스트리밍               │
└────────────────────────────────────────────────┘
```

API 키는 평문으로 디스크에 저장하지 않고 OS Credential Manager(Windows) / Keychain(macOS) / Secret Service(Linux)에 위임합니다.

---

## 로드맵

### v0.1 MVP (진행 중)

- [x] Phase 0: 스캐폴딩 + OpenRouter 단일턴
- [ ] Markdown + 코드 syntax highlight
- [ ] 다회 채팅 + 세션 관리
- [ ] 도구 호출 루프 (read/write/glob/grep)
- [ ] Plan ↔ Build 모드
- [ ] 명령 팔레트 (Cmd/Ctrl+K)
- [ ] PTY 터미널 (`portable-pty` + `xterm.js`)
- [ ] 접근성 기본 구현

### v0.5 폴리시

- [ ] 고급 애니메이션 / 12개 테마 프리셋
- [ ] Diff Preview (side-by-side / unified)
- [ ] LSP 통합

### v1.0 출시

- [ ] Pro 결제 / 자동 업데이트
- [ ] 팀 세션 공유
- [ ] Self-hosted / SSO

---

## 기여

지금 단계는 매우 초기라 기여보다는 이슈/제안 환영합니다.

## 라이선스

추후 결정 (오픈소스 + 상용 듀얼 라이선스 검토 중).
