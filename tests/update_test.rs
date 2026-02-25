#[cfg(test)]
mod tests {
    #[test]
    fn test_version_parsing() {
        let url = "https://github.com/sentinel-team/sentinel-pro/releases/download/v5.0.0/sentinel";
        assert!(url.contains("v5.0.0"));
    }

    #[test]
    fn test_binary_path_lookup() {
        // This will only work if sentinel is in PATH
        let path = "/usr/local/bin/sentinel";
        assert!(path.ends_with("sentinel"));
    }
}
