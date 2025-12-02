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

        let result = editor.replace_string(params).await;

        // Should return an error for out of bounds line range
        assert!(result.is_err());

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

    #[tokio::test]
    async fn test_replace_empty_string_returns_error() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello world").await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "".to_string(), // Empty string
            new_str: "replacement".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await;

        // Should return an error
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_replace_with_empty_string() {
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
            old_str: "world".to_string(),
            new_str: "".to_string(), // Replace with empty string (deletion)
            occurrence: Some(ReplaceOccurrence::First),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 1);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "Hello ");
    }

    #[tokio::test]
    async fn test_multiline_replacement() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Line 1\nLine 2\nLine 3\nLine 4";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "Line 2\nLine 3".to_string(), // Multi-line replacement
            new_str: "Merged Line".to_string(),
            occurrence: Some(ReplaceOccurrence::First),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 1);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "Line 1\nMerged Line\nLine 4");
    }

    #[tokio::test]
    async fn test_special_characters_replacement() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Hello $world$ [test] (data)";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "$world$".to_string(),
            new_str: "{universe}".to_string(),
            occurrence: Some(ReplaceOccurrence::First),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 1);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "Hello {universe} [test] (data)");
    }

    #[tokio::test]
    async fn test_file_not_exists() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("nonexistent.txt");

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "test".to_string(),
            new_str: "replacement".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(!result.success);
        assert_eq!(result.replacements_made, 0);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_invalid_occurrence_index() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "apple banana apple";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        // Try to replace 5th occurrence when only 2 exist
        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "apple".to_string(),
            new_str: "orange".to_string(),
            occurrence: Some(ReplaceOccurrence::Index(5)),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await;

        // Should return an error
        assert!(result.is_err());

        // File should remain unchanged
        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, original_content);
    }

    #[tokio::test]
    async fn test_line_range_invalid_start_line() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Line 1\nLine 2\nLine 3";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        // Start line 0 is invalid (1-based indexing)
        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "Line".to_string(),
            new_str: "Row".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: Some((0, 2)),
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await;

        // Should return an error
        assert!(result.is_err());

        // File should remain unchanged
        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, original_content);
    }

    #[tokio::test]
    async fn test_line_range_inverted() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Line 1\nLine 2\nLine 3";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        // Start line > end line
        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "Line".to_string(),
            new_str: "Row".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: Some((3, 1)),
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await;

        // Should return an error
        assert!(result.is_err());

        // File should remain unchanged
        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, original_content);
    }

    #[tokio::test]
    async fn test_large_content() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create large content with repeated pattern
        let mut large_content = String::new();
        for i in 0..1000 {
            large_content.push_str(&format!("Line {}: pattern to replace\n", i));
        }
        fs::write(&file_path, &large_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "pattern to replace".to_string(),
            new_str: "REPLACED".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 1000);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert!(new_content.contains("REPLACED"));
        assert!(!new_content.contains("pattern to replace"));
    }

    #[tokio::test]
    async fn test_preview_creation() {
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
            create_backup: Some(false),
            dry_run: Some(true),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert!(result.preview.is_some());
        let preview = result.preview.unwrap();
        assert!(preview.contains("-") || preview.contains("+"));
    }

    #[tokio::test]
    async fn test_case_insensitive_multiple_variants() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Test test TEST TeSt";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            case_sensitive: false,
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "test".to_string(),
            new_str: "RESULT".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 4);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "RESULT RESULT RESULT RESULT");
    }

    #[tokio::test]
    async fn test_line_range_boundary_conditions() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Line 1: foo\nLine 2: foo\nLine 3: foo";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        // Replace only in first line
        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "foo".to_string(),
            new_str: "bar".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: Some((1, 1)),
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 1);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "Line 1: bar\nLine 2: foo\nLine 3: foo");
    }
}
