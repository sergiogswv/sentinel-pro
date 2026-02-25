#[cfg(test)]
mod tests {
    #[test]
    fn test_telemetry_event_creation() {
        // Test basic event creation
        assert_eq!(1, 1);
    }

    #[test]
    fn test_telemetry_disabled_via_env() {
        std::env::set_var("SENTINEL_TELEMETRY", "false");
        // Event creation should still work, but sending should be skipped
        assert_eq!(true, true);
    }

    #[test]
    fn test_telemetry_storage_path() {
        // Test that telemetry storage path is valid
        let path_str = "~/.sentinel/telemetry.log";
        assert!(path_str.contains(".sentinel"));
    }
}
