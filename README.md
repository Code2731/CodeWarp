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
- [x] **Phase 2-4** — 자동 스크롤, 부분 텍스트 선택 복사, Markdown 렌더링 + 모델/세션/스크롤 위치 영구 저장
- [x] **Phase 2-5** — 도구 호출 루프 (read_file/write_file/glob/grep/run_command), diff 뷰, 인라인 confirm
- [x] **Phase 2-6** — Plan/Build 모드, 슬래시 명령, 명령 팔레트(Ctrl+K), Custom Theme(Warp 톤), 키보드 단축키
- [x] **Phase 2-7** — 멀티 provider (OpenRouter + Tabby 동시 활성), [KO] 한국어 친화 태그
- [x] **Phase 2-8** — chat_stream provider 라우팅 (선택 모델의 provider로 분기)
- [x] **Phase 2-9** — 모델 매니저 (HF 모델 다운로드 + 추천 프리셋, Show/Hide 토큰)
- [x] **UX-A~E** — UX 감사 기반 PR 묶음: 응답 중지·삭제 확인·블록 모델 ID·빈 채팅 onboarding·Settings 안내·인라인 diff 토글·개별 거부·도구 결과 chip·모델 픽커 정밀화·시각 강조·Right panel 활용
- [x] **P4-1 / P4-3** — 코드 블록 Apply (Cursor 영감), 응답 regenerate (↻) + edit-and-resend (✎) (TDD)
- [x] **Phase 2-A** — generic OpenAI 호환 provider (Tabby/xLLM/Ollama/llama-server 등 어떤 endpoint든 등록 가능, 사용자 라벨 명명: `[xLLM]` / `[Local]` 등)
- [x] **Phase 2-B** — endpoint 활성화 시각 indicator (●연결됨/끊김/미시도)
- [x] **Phase 2-C** — inference 서버 spawn 관리: 엔진 dropdown(xLLM/vLLM/llama-server/Tabby/Ollama/Custom) + 받은 모델 dropdown + 바이너리 경로 picker(PATH 의존 제거) + 자동 명령 합성 + child process 관리(시작/중지/로그/auto ping)
- [x] **Phase 2-D** — EXL2 프리셋 원클릭 다운로드: `hf::download_repo`에 `revision`·`folder_name` 파라미터 추가(HF 브랜치별 bpw 선택), 모델 매니저에 EXL2 추천 프리셋 목록(1B~12B, VRAM 표시) + 버튼 클릭 즉시 다운로드 시작
- [x] **Tests** — 회고적 + TDD 누적 118 tests
- [x] **P4-2 / P4-5** — @-mention 파일 컨텍스트 + Drag & drop 파일 첨부
- [x] **refactor** — main.rs(5544줄) → main + update + view 3파일 분리
- [x] **MCP** — stdio MCP 서버 클라이언트: 서버 등록/제거·tools/list·tools/call 비동기 라우팅·tool_definitions 동적 합산
- [x] **Phase 2-10** — PTY 터미널: portable-pty(ConPTY/POSIX) + 라인 입력 모드 + ANSI strip + 하단 토글 패널(Ctrl+\`) + 121 tests passed

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
│  ─ tabby.rs      : OpenAI 호환 generic 라우터 │
│  ─ hf.rs         : HF Hub 다운로드 (EXL2 등) │
│  ─ session.rs    : 멀티 세션 영구 저장        │
│  ─ tools.rs      : read/write/glob/grep/run   │
│  ─ tokio         : async runtime              │
│  ─ async-stream  : Stream<ChatEvent/DlEvent>  │
└──────────────────────────────────────────────┘
```

Tauri/webview/JS 의존성 일체 없음. 단일 Cargo 프로젝트.

## 빠른 시작

### 사전 요구사항

- Rust 1.80+ (`rustup`)
- Windows / macOS / Linux 데스크톱
- OpenRouter API 키 — <https://openrouter.ai/keys> (선택)
- 또는 로컬 Tabby 서버 — <https://tabby.tabbyml.com/> (선택)

OpenRouter와 Tabby는 동시에 활성화 가능하며, 모델 셀렉터에서 메시지마다 자유 선택할 수 있습니다.

### 개발 모드

```bash
git clone https://github.com/Code2731/CodeWarp.git
cd CodeWarp
cargo run
```

처음 실행하면 Settings 화면이 뜹니다. OpenRouter 키를 입력하면 OS Credential Manager(Windows) / Keychain(macOS) / Secret Service(Linux)에 저장되며, 모델 리스트가 자동으로 페치됩니다.

이후 실행부터는 키와 마지막으로 선택한 모델이 자동으로 복원됩니다.

#### OpenAI 호환 endpoint 연결 (선택)

Tabby / xLLM / vLLM / llama-server / Ollama / TabbyAPI 등 OpenAI 호환 inference endpoint면 무엇이든 등록 가능. Settings → "OpenAI 호환 endpoint" 섹션에:
- **라벨** — 모델 셀렉터에 prefix로 표시 (예: `xLLM` → `[xLLM]`)
- **base URL** — 예: `http://localhost:9000` (`/v1` 자동 추가)
- **토큰** — 99% 케이스에서 불필요 (Show/Hide 토글로 숨김)

저장 시 자동으로 `/v1/models` ping → 모델 셀렉터에 `[<라벨>]` prefix로 추가.

#### inference 서버 spawn 관리 (선택)

별도 터미널에서 띄우는 게 친화적이지 않으니 CodeWarp가 child process로 spawn 관리:
- 엔진 dropdown — xLLM / vLLM / llama-server / Tabby / Ollama / Custom
- 모델 dropdown — 모델 매니저로 받은 폴더 자동 스캔
- 바이너리 경로 picker — PATH에 없을 때 절대 경로 지정 (예: winget으로 설치한 Tabby의 실제 폴더)
- 시작 → 자동 명령 합성 + spawn + 5초 후 endpoint 자동 ping + child 종료 시 endpoint 자동 끊김 표시
- 앱 종료 시 child도 같이 cleanup (좀비 방지)

모델 셀렉터 표시 예:
- `[OR][KO] gpt-4o  $5.00/$15.00` — OpenRouter, 한국어 친화 모델
- `[xLLM] qwen2.5-coder-7b  free` — 자체 띄운 xLLM endpoint
- `[Ollama] qwen2.5-coder:7b  free` — 외부 Ollama daemon
- `[Tabby] StarCoder-1B  free` — winget 설치 Tabby

`[KO]` 태그는 한국어 토크나이저 친화로 알려진 모델(claude/gpt-4o/qwen2.5/exaone/solar/deepseek-v3 등)에 자동 표시 — 영어 위주 BPE보다 한국어 비용이 낮은 모델을 빠르게 식별.

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

## Harness

Quality harness is now part of the default workflow.

- Local checks: `scripts/harness.ps1` (Windows), `scripts/harness.sh` (Linux/macOS)
- CI uses the same harness entry path
- Recommended hooks:
  - `pre-commit`: `cargo fmt -- --check`
  - `pre-push`: harness (`fmt + check + test`)

Hook installation:

```powershell
pwsh -File scripts/install-hooks.ps1
```

```bash
bash scripts/install-hooks.sh
```

More details: `docs/HARNESS.md`
