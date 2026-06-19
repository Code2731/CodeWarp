// ── Presets ─────────────────────────────────────────────────────────

/// 추천 프리셋 — 클릭 시 hf_repo_input에 채움.
#[derive(Debug)]
pub(crate) struct ModelPreset {
    pub(crate) repo_id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) note: &'static str,
}

pub(crate) const MODEL_PRESETS: &[ModelPreset] = &[
    ModelPreset {
        repo_id: "Qwen/Qwen2.5-Coder-7B-Instruct",
        label: "Qwen2.5-Coder 7B Instruct",
        note: "코딩 + 한국어 친화 (xLLM/vLLM)",
    },
    ModelPreset {
        repo_id: "Qwen/Qwen2.5-7B-Instruct",
        label: "Qwen2.5 7B Instruct",
        note: "범용 + 한국어 친화 (xLLM/vLLM)",
    },
    ModelPreset {
        repo_id: "LGAI-EXAONE/EXAONE-3.5-7.8B-Instruct",
        label: "EXAONE 3.5 7.8B",
        note: "한국어 특화 (LG AI)",
    },
    ModelPreset {
        repo_id: "upstage/SOLAR-10.7B-Instruct-v1.0",
        label: "SOLAR 10.7B",
        note: "한국어 친화 (Upstage)",
    },
    ModelPreset {
        repo_id: "deepseek-ai/DeepSeek-Coder-V2-Lite-Instruct",
        label: "DeepSeek-Coder V2 Lite",
        note: "코딩 (16B-MoE 활성 2.4B)",
    },
];

/// EXL2 프리셋 — TabbyAPI용. 클릭하면 해당 branch를 바로 다운로드.
#[derive(Debug)]
pub(crate) struct Exl2Preset {
    pub(crate) repo_id: &'static str,
    pub(crate) revision: &'static str,
    pub(crate) folder_name: &'static str,
    pub(crate) label: &'static str,
    pub(crate) note: &'static str,
    pub(crate) vram: &'static str,
}

pub(crate) const EXL2_PRESETS: &[Exl2Preset] = &[
    Exl2Preset {
        repo_id: "turboderp/Llama-3.2-1B-Instruct-exl2",
        revision: "4.0bpw",
        folder_name: "Llama-3.2-1B-Instruct-4.0bpw",
        label: "Llama 3.2 1B Instruct",
        note: "검증·테스트용 초소형",
        vram: "~600MB",
    },
    Exl2Preset {
        repo_id: "turboderp/Llama-3.2-3B-Instruct-exl2",
        revision: "3.5bpw",
        folder_name: "Llama-3.2-3B-Instruct-3.5bpw",
        label: "Llama 3.2 3B Instruct",
        note: "소형 범용",
        vram: "~1.8GB",
    },
    Exl2Preset {
        repo_id: "turboderp/Llama-3.1-8B-Instruct-exl2",
        revision: "4.0bpw",
        folder_name: "Llama-3.1-8B-Instruct-4.0bpw",
        label: "Llama 3.1 8B Instruct 4bpw",
        note: "RTX 3080 최적 균형",
        vram: "~5GB",
    },
    Exl2Preset {
        repo_id: "turboderp/Llama-3.1-8B-Instruct-exl2",
        revision: "6.0bpw",
        folder_name: "Llama-3.1-8B-Instruct-6.0bpw",
        label: "Llama 3.1 8B Instruct 6bpw",
        note: "품질 우선 (RTX 3080 10GB 내)",
        vram: "~7.5GB",
    },
    Exl2Preset {
        repo_id: "turboderp/gemma-2-9b-it-exl2",
        revision: "4.0bpw",
        folder_name: "Gemma-2-9B-it-4.0bpw",
        label: "Gemma 2 9B Instruct",
        note: "Google 범용 (강력한 instruction following)",
        vram: "~5.5GB",
    },
    Exl2Preset {
        repo_id: "turboderp/gemma-3-12b-it-exl2",
        revision: "4.0bpw",
        folder_name: "Gemma-3-12B-it-4.0bpw",
        label: "Gemma 3 12B Instruct",
        note: "최신 Gemma 3 (멀티모달 지원)",
        vram: "~7GB",
    },
];
