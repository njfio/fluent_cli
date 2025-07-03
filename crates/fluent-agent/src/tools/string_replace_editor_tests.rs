#[cfg(test)]
mod comprehensive_tests {
    use super::super::string_replace_editor::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_replace_occurrence_first() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "apple banana apple cherry apple";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "apple".to_string(),
            new_str: "orange".to_string(),
            occurrence: Some(ReplaceOccurrence::First),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 1);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "orange banana apple cherry apple");
    }

    #[tokio::test]
    async fn test_replace_occurrence_last() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "apple banana apple cherry apple";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "apple".to_string(),
            new_str: "orange".to_string(),
            occurrence: Some(ReplaceOccurrence::Last),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 1);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "apple banana apple cherry orange");
    }

    #[tokio::test]
    async fn test_replace_occurrence_indexed() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "apple banana apple cherry apple";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "apple".to_string(),
            new_str: "orange".to_string(),
            occurrence: Some(ReplaceOccurrence::Index(2)), // Second occurrence
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 1);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "apple banana orange cherry apple");
    }

    #[tokio::test]
    async fn test_backup_creation() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Hello world";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "Hello".to_string(),
            new_str: "Hi".to_string(),
            occurrence: Some(ReplaceOccurrence::First),
            line_range: None,
            create_backup: Some(true),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 1);
        assert!(result.backup_path.is_some());

        // Check that backup file exists and contains original content
        let backup_path = result.backup_path.unwrap();
        assert!(fs::metadata(&backup_path).await.is_ok());
        
        let backup_content = fs::read_to_string(&backup_path).await.unwrap();
        assert_eq!(backup_content, original_content);

        // Check that original file was modified
        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "Hi world");
    }

    #[tokio::test]
    async fn test_case_sensitivity() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Hello HELLO hello HeLLo";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            case_sensitive: false, // Case insensitive
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "hello".to_string(),
            new_str: "hi".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 4); // All variants should be replaced

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "hi hi hi hi");
    }

    #[tokio::test]
    async fn test_case_sensitive() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Hello HELLO hello HeLLo";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            case_sensitive: true, // Case sensitive
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "hello".to_string(),
            new_str: "hi".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 1); // Only exact match

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "Hello HELLO hi HeLLo");
    }

    #[tokio::test]
    async fn test_line_range_out_of_bounds() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Line 1\nLine 2\nLine 3";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        // Try to replace in lines 5-10 (out of bounds)
        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "Line".to_string(),
            new_str: "Row".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: Some((5, 10)),
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 0); // No replacements should be made

        // File should remain unchanged
        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, original_content);
    }

    #[tokio::test]
    async fn test_empty_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("empty.txt");

        fs::write(&file_path, "").await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "anything".to_string(),
            new_str: "something".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 0);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "");
    }

    #[tokio::test]
    async fn test_no_matches_found() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Hello world";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "nonexistent".to_string(),
            new_str: "replacement".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 0);

        // File should remain unchanged
        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, original_content);
    }
}
