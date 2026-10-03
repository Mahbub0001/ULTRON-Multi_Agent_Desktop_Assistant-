#[cfg(test)]
mod tests {
    use crate::dsl::{DslParser, InterpreterConfig};
    use crate::recorder::RecorderConfig;

    #[test]
    fn test_parser_creation() {
        let parser = DslParser::new();
        assert!(parser.include_paths.contains(&std::path::PathBuf::from(".")));
    }

    #[test]
    fn test_parser_with_include_paths() {
        let parser = DslParser::new().with_include_paths(vec![
            std::path::PathBuf::from("/custom/path"),
            std::path::PathBuf::from("./includes"),
        ]);
        assert_eq!(parser.include_paths.len(), 2);
    }

    #[test]
    fn test_interpreter_config() {
        let config = InterpreterConfig {
            max_execution_time_ms: 60000,
            max_loop_iterations: 1000,
            max_recursion_depth: 50,
            default_step_timeout_ms: 5000,
        };
        assert_eq!(config.max_execution_time_ms, 60000);
        assert_eq!(config.max_loop_iterations, 1000);
        assert_eq!(config.max_recursion_depth, 50);
        assert_eq!(config.default_step_timeout_ms, 5000);
    }

    #[test]
    fn test_recorder_config() {
        let config = RecorderConfig {
            capture_keyboard: false,
            capture_mouse: true,
            capture_delays: false,
            max_duration: Some(std::time::Duration::from_secs(60)),
            stop_key: Some("ctrl+shift+q".to_string()),
        };
        assert!(!config.capture_keyboard);
        assert!(config.capture_mouse);
        assert!(!config.capture_delays);
        assert_eq!(config.max_duration, Some(std::time::Duration::from_secs(60)));
        assert_eq!(config.stop_key, Some("ctrl+shift+q".to_string()));
    }
}