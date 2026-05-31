# CodeWarp Architecture

CodeWarp는 Iced 프레임워크 기반의 Rust 네이티브 AI 코딩 데스크톱 앱입니다. Tauri/WebView 의존성 없이 단일 Cargo 프로젝트로 빌드됩니다.

## Core Architecture: Iced Elm Pattern

CodeWarp는 Iced의 Elm 아키텍처를 따릅니다:

```
State (App)  ←→  Message  ←→  update()  ←→  Task  ←→  view()
```

- **State**: `App` 구조체가 모든 애플리케이션 상태를 보유
- **Message**: `Message` enum이 모든 가능한 이벤트를 정의
- **update**: `App::update(message) -> Task<Message>`가 상태 전이를 처리
- **view**: `App::view()`가 현재 상태 기반 UI를 렌더링
- **Task**: 비동기 작업의 반환 타입 (`Task::none()`, `Task::perform()`, `Task::done()`, `Task::run()`)

## File Structure

```
src/
├── main.rs          # App struct, Message enum, module declarations, tests
├── update.rs        # App::update() 메인 디스패치 + 모든 helper 메서드
├── view.rs          # App::view() UI 렌더링 (settings, chat, sidebar, right panel)
├── session.rs       # 세션 직렬화/역직렬화, 디스크 저장/로드
├── openrouter.rs    # OpenRouter HTTP/SSE 클라이언트, 모델 목록, 채팅 스트림
├── tabby.rs         # OpenAI-호환 클라이언트, 모델 목록, 채팅 스트림
├── hf.rs            # HuggingFace 다운로드, EXL2 프리셋, revision 처리
├── tools.rs         # 도구 호출 시스템 (read_file, write_file, glob, grep, run_command)
├── mcp.rs           # MCP 클라이언트 (stdio 서버 연결, tools/list, tools/call)
├── pty.rs           # PTY 터미널 (portable-pty 기반)
├── keystore.rs      # 크레덴셜 관리 (Windows Credential Manager, macOS Keychain, Linux Secret Service)
└── lib.rs           # 공유 타입/상수
```

## App State Composition

`App` 구조체는 `src/main.rs`에 정의되어 있으며, 상태 그룹으로 분리 가능:

- **Core UI**: `ui: UiState` (설정 패널, 명령 팔레트, 삭제 확인 등)
- **Model Filter**: `model_filter: ModelFilterState` (코딩/추론/일반 필터, 정렬 모드, 즐겨찾기)
- **MCP Input**: `mcp_input: McpInputState` (MCP 서버 이름/명령 입력)
- **Provider State**: `tabby_url_input`, `tabby_token_input`, `openai_compat_label`, `hf_token_input`
- **Inference State**: `inference_engine`, `inference_selected_model`, `inference_port_input`, `inference_binary_path`
- **Chat State**: `conversation`, `blocks`, `pending_tool_calls`, `pending_write_calls`, `streaming_block_id`
- **Session State**: `current_session_id`, `current_session_title`, `inactive_sessions`
- **Model State**: `model_options`, `selected_model`, `selected_model_provider`, `usage`

## Async Runtime Model

CodeWarp는 Tokio 런타임을 사용하며, Iced의 `Task` 추상화와 통합됩니다:

- `Task::none()` — 상태만 변경, 비동기 작업 없음
- `Task::done(Message)` — 즉시 다른 Message를 디스패치
- `Task::perform(async_fn, msg_fn)` — 비동기 함수 실행 후 결과를 Message로 변환
- `Task::run(stream, msg_fn)` — 스트림에서 이벤트를 수신하여 Message로 변환

채팅 스트림은 `Task::run`으로 SSE 이벤트를 수신하고 `Message::ChatChunk`로 토큰을 전달합니다.

## Update Pipeline

`App::update()`는 `src/update.rs`에서 모든 Message를 라우팅합니다. 순수 UI/state 변경 arms는 helper 메서드로 분리되어 있습니다:

```
Message::ToggleFavorite => self.toggle_favorite(),

fn toggle_favorite(&mut self) -> Task<Message> {
    // ... state mutation ...
    Task::none()
}
```

비동기 작업은 두 개의 arm으로 분리됩니다:
1. 시작 arm: `Task::perform(async, Message::Result)`
2. 결과 arm: `Message::Result(r) => { /* handle */ }`

자세한 내용은 [UpdatePipeline.md](UpdatePipeline.md) 참조.

## Subsystem Interactions

- **OpenRouter**: 외부 API, API 키 필요, SSE 스트리밍, 모델 목록 fetch
- **Tabby/OpenAI-compat**: 로컬/원격 엔드포인트, 사용자 등록, 커스텀 라벨
- **HF Download**: HuggingFace에서 모델 다운로드, EXL2 프리셋 지원
- **MCP**: stdio 기반 MCP 서버 연결, 동적 tool 정의 로드
- **PTY**: portable-pty 기반 터미널 에뮬레이션
- **Session**: 로컬 JSON 파일 기반 세션 영속화
- **Keystore**: OS 크레덴셜 매니저에 API 키 저장

## Security Model

- API 키는 평문으로 디스크에 저장하지 않음 (OS Credential Manager 사용)
- 키는 코드/로그/git 어디에도 출력되지 않음
- `keystore.rs`가 모든 크레덴셜 I/O를 캡슐화
