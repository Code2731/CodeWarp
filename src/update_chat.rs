// update_chat.rs — Chat-related App update methods (main.rs child module)
use super::{AgentMode, App, Arc, ChatMessage};

impl App {
    pub(crate) fn ensure_system_message(&mut self) {
        let mode_block = match self.agent_mode {
            AgentMode::Plan => {
                "현재 모드: Plan (분석/계획 전용)\n\
                Plan 모드에서는 read_file/glob/grep으로 코드를 조사하고 변경 계획만 \
                제시하세요. 실제 파일 변경이나 명령 실행은 Build 모드에서만 가능하므로, \
                계획에 '필요한 변경'을 명확히 적고 사용자가 Build로 전환하기를 기다리세요.\n\n"
            }
            AgentMode::Build => {
                "현재 모드: Build (실행 가능)\n\
                Build 모드에서는 write_file/run_command를 사용해 실제 변경을 적용할 수 \
                있습니다. 단, 두 도구 모두 사용자 승인을 거치므로 부담 없이 호출하세요.\n\n"
            }
        };
        let prompt = format!(
            "당신은 CodeWarp의 코딩 어시스턴트입니다.\n\n\
            작업 디렉토리: '{}'\n\n\
            {}\
            사용 가능한 도구 (적극적으로 호출하세요):\n\
            - read_file(path): 파일 내용 읽기 (즉시 실행)\n\
            - write_file(path, content): 파일 작성/덮어쓰기 (Build 모드 + 사용자 승인)\n\
            - run_command(command): 셸 명령 실행 (Build 모드 + 사용자 승인)\n\
            - glob(pattern): 패턴 매칭 파일 리스트 (예: '**/*.rs', 'examples/**/*')\n\
            - grep(pattern): 정규식으로 모든 파일 검색\n\n\
            규칙:\n\
            1. 파일 시스템을 살펴봐야 할 때는 '확인하겠습니다' 같은 말 없이 즉시 도구를 호출하세요.\n\
            2. 새 파일을 만들기 전에 glob으로 기존 구조를 먼저 확인하세요.\n\
            3. 모든 path 인자는 작업 디렉토리 기준 상대 경로 (절대 경로 거부).\n\
            4. 도구 결과를 받은 뒤 그것을 근거로 한국어로 답하세요.\n\
            5. **마크다운 형식 제약** (한국어 폰트 한계): italic(*text* 또는 _text_)은 \
            사용하지 마세요. 강조는 오직 **굵게**만 사용. 별표 한 개로 감싸지 말고, \
            정말 강조가 필요하면 두 개로 감싸세요.\n\
            6. **Apply 가능한 코드 블록**: 사용자가 그대로 파일에 적용할 수 있도록, \
            새 파일/덮어쓸 파일의 코드 블록은 첫 줄에 다음 주석을 포함하세요:\n\
            - Rust/JS/C 계열: `// path: 상대경로`\n\
            - Python/shell/yaml: `# path: 상대경로`\n\
            예) ```rust\\n// path: src/foo.rs\\nfn main() {{}}\\n```\n\
            그러면 코드 블록 옆에 'Apply' 버튼이 노출되어 사용자가 한 번에 적용할 수 있습니다. \
            단순 예시 코드(개념 설명용)에는 path 주석을 넣지 마세요.",
            self.cwd.display(),
            mode_block,
        );
        if let Some(first) = Arc::make_mut(&mut self.conversation).first_mut()
            && first.role == "system"
        {
            first.content = Some(prompt);
            return;
        }
        Arc::make_mut(&mut self.conversation).insert(
            0,
            ChatMessage {
                role: "system".into(),
                content: Some(prompt),
                ..Default::default()
            },
        );
    }
}
