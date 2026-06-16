#[derive(Debug, serde::Deserialize)]
pub(super) struct ReadFileArgs {
    pub(super) path: String,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct GlobArgs {
    pub(super) pattern: String,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct GrepArgs {
    pub(super) pattern: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct WriteFileArgs {
    pub path: String,
    pub content: String,
}

impl WriteFileArgs {
    pub fn parse(arguments_json: &str) -> Result<Self, String> {
        serde_json::from_str(arguments_json).map_err(|e| e.to_string())
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct RunCommandArgs {
    pub command: String,
}

impl RunCommandArgs {
    pub fn parse(arguments_json: &str) -> Result<Self, String> {
        serde_json::from_str(arguments_json).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_file_args_parse_ok() {
        let args = WriteFileArgs::parse(r#"{"path":"a.rs","content":"x"}"#).unwrap();
        assert_eq!(args.path, "a.rs");
        assert_eq!(args.content, "x");
    }

    #[test]
    fn write_file_args_parse_missing_field() {
        assert!(WriteFileArgs::parse(r#"{"path":"a.rs"}"#).is_err());
        assert!(WriteFileArgs::parse(r#"{"content":"x"}"#).is_err());
    }

    #[test]
    fn write_file_args_parse_invalid_json() {
        assert!(WriteFileArgs::parse("not json").is_err());
        assert!(WriteFileArgs::parse("").is_err());
    }

    #[test]
    fn run_command_args_parse_ok() {
        let args = RunCommandArgs::parse(r#"{"command":"echo hi"}"#).unwrap();
        assert_eq!(args.command, "echo hi");
    }

    #[test]
    fn run_command_args_parse_missing() {
        assert!(RunCommandArgs::parse(r#"{}"#).is_err());
        assert!(RunCommandArgs::parse("not json").is_err());
    }
}
