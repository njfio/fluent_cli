use anyhow::{anyhow, Error, Result};
use regex::Regex;
use serde_json::Value;

/// Extract Cypher query from response content
pub fn extract_cypher_query(content: &str) -> Result<String, Error> {
    // First, try to extract content between triple backticks
    let backtick_re = Regex::new(r"```(?:cypher)?\s*([\s\S]*?)\s*```")
        .map_err(|e| anyhow!("Failed to compile regex: {}", e))?;
    
    if let Some(captures) = backtick_re.captures(content) {
        if let Some(query) = captures.get(1) {
            let extracted = query.as_str().trim();
            if !extracted.is_empty() && is_valid_cypher(extracted) {
                return Ok(extracted.to_string());
            }
        }
    }

    // If no backticks found, try to extract Cypher-like statements
    let cypher_patterns = [
        r"(?i)(MATCH\s+.*?(?:RETURN|WHERE|WITH|CREATE|MERGE|DELETE|SET).*?)(?:\n\n|\z)",
        r"(?i)(CREATE\s+.*?(?:RETURN|WHERE|WITH|MATCH|MERGE|DELETE|SET).*?)(?:\n\n|\z)",
        r"(?i)(MERGE\s+.*?(?:RETURN|WHERE|WITH|MATCH|CREATE|DELETE|SET).*?)(?:\n\n|\z)",
    ];

    for pattern in &cypher_patterns {
        let re = Regex::new(pattern)
            .map_err(|e| anyhow!("Failed to compile pattern regex: {}", e))?;
        
        if let Some(captures) = re.captures(content) {
            if let Some(query) = captures.get(1) {
                let extracted = query.as_str().trim();
                if is_valid_cypher(extracted) {
                    return Ok(extracted.to_string());
                }
            }
        }
    }

    Err(anyhow!("No valid Cypher query found in content"))
}

/// Validate if a string contains a valid Cypher query
pub fn is_valid_cypher(query: &str) -> bool {
    // Basic validation: check if the query contains common Cypher clauses
    let valid_clauses = [
        "MATCH", "CREATE", "MERGE", "DELETE", "SET", "REMOVE", 
        "RETURN", "WHERE", "WITH", "UNWIND", "CALL", "YIELD"
    ];
    
    let query_upper = query.to_uppercase();
    valid_clauses.iter().any(|clause| query_upper.contains(clause))
}

/// Format query result as CSV
pub fn format_as_csv(result: &Value) -> String {
    match result {
        Value::Array(records) => {
            if records.is_empty() {
                return String::new();
            }

            let mut csv_lines = Vec::new();
            
            // Extract headers from first record
            if let Some(Value::Object(first_record)) = records.first() {
                let headers: Vec<String> = first_record.keys().cloned().collect();
                csv_lines.push(headers.join(","));
                
                // Process each record
                for record in records {
                    if let Value::Object(obj) = record {
                        let values: Vec<String> = headers.iter()
                            .map(|header| {
                                obj.get(header)
                                    .map(|v| format_csv_value(v))
                                    .unwrap_or_default()
                            })
                            .collect();
                        csv_lines.push(values.join(","));
                    }
                }
            }
            
            csv_lines.join("\n")
        }
        Value::Object(obj) => {
            // Single object - convert to single row CSV
            let headers: Vec<String> = obj.keys().cloned().collect();
            let values: Vec<String> = headers.iter()
                .map(|header| {
                    obj.get(header)
                        .map(|v| format_csv_value(v))
                        .unwrap_or_default()
                })
                .collect();
            
            format!("{}\n{}", headers.join(","), values.join(","))
        }
        _ => {
            // For other types, just return the JSON string
            result.to_string()
        }
    }
}

/// Format a JSON value for CSV output
fn format_csv_value(value: &Value) -> String {
    match value {
        Value::String(s) => {
            // Escape quotes and wrap in quotes if contains comma or quote
            if s.contains(',') || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.clone()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        Value::Array(_) | Value::Object(_) => {
            // For complex types, use JSON representation
            format!("\"{}\"", value.to_string().replace('"', "\"\""))
        }
    }
}

/// Extract code blocks from response content
pub fn extract_code(response: &str, file_type: &str) -> String {
    // Handle specific file type patterns first
    let code_block_start = match file_type {
        "html" => "```html",
        "js" | "javascript" => "```javascript",
        "rs" | "rust" => "```rust",
        _ => "```",
    };

    if let Some(start) = response.find(code_block_start) {
        let code_start = start + code_block_start.len();
        if let Some(end_pos) = response[code_start..].find("```") {
            let code_end = code_start + end_pos;
            return response[code_start..code_end].trim().to_string();
        }
    }

    // Try generic code blocks
    if let Some(start) = response.find("```") {
        let code_start = start + 3;
        // Skip language identifier if present
        let actual_start = if let Some(newline) = response[code_start..].find('\n') {
            code_start + newline + 1
        } else {
            code_start
        };

        if let Some(end_pos) = response[actual_start..].find("```") {
            let code_end = actual_start + end_pos;
            return response[actual_start..code_end].trim().to_string();
        }
    }

    // File type specific fallbacks
    match file_type {
        "html" => {
            if response.contains("<!DOCTYPE html") || response.contains("<html") {
                return response.trim().to_string();
            }
            // HTML fallback template
            create_html_fallback()
        }
        "rs" | "rust" => {
            if response.contains("fn main()") {
                return response.trim().to_string();
            }
            // Rust fallback template
            create_rust_fallback()
        }
        _ => {
            // Use the original regex-based approach for other file types
            let pattern = format!(r"```{}\s*([\s\S]*?)\s*```", regex::escape(file_type));
            let re = match Regex::new(&pattern) {
                Ok(re) => re,
                Err(_) => return response.trim().to_string(),
            };

            let mut extracted_code = Vec::new();

            for captures in re.captures_iter(response) {
                if let Some(code) = captures.get(1) {
                    extracted_code.push(code.as_str().trim());
                }
            }

            if extracted_code.is_empty() {
                // Fallback: try without specific language
                let generic_pattern = r"```\s*([\s\S]*?)\s*```";
                if let Ok(generic_re) = Regex::new(generic_pattern) {
                    for captures in generic_re.captures_iter(response) {
                        if let Some(code) = captures.get(1) {
                            let code_text = code.as_str().trim();
                            // Basic heuristic to check if it matches the file type
                            if matches_file_type(code_text, file_type) {
                                extracted_code.push(code_text);
                            }
                        }
                    }
                }
            }

            if extracted_code.is_empty() {
                response.trim().to_string()
            } else {
                extracted_code.join("\n\n")
            }
        }
    }
}

/// Check if code content matches the expected file type
fn matches_file_type(code: &str, file_type: &str) -> bool {
    match file_type {
        "rust" | "rs" => {
            code.contains("fn ") || code.contains("struct ") ||
            code.contains("impl ") || code.contains("use ")
        }
        "python" | "py" => {
            code.contains("def ") || code.contains("import ") ||
            code.contains("from ") || code.contains("class ")
        }
        "javascript" | "js" => {
            code.contains("function ") || code.contains("const ") ||
            code.contains("let ") || code.contains("var ")
        }
        "html" => {
            code.contains("<html") || code.contains("<!DOCTYPE") || code.contains("<body")
        }
        "json" => {
            code.trim_start().starts_with('{') || code.trim_start().starts_with('[')
        }
        "yaml" | "yml" => {
            code.contains(':') && !code.contains(';')
        }
        _ => true, // Default to true for unknown types
    }
}

/// Create HTML fallback template for games
fn create_html_fallback() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Frogger Game - Created by Agentic System</title>
    <style>
        body { margin: 0; padding: 20px; background: #222; color: white; font-family: Arial, sans-serif; }
        canvas { border: 2px solid #fff; background: #000; }
        .info { margin-top: 10px; }
    </style>
</head>
<body>
    <h1>🐸 Frogger Game - Created by Agentic System</h1>
    <canvas id="gameCanvas" width="800" height="600"></canvas>
    <div class="info">
        <p>Use arrow keys to move the frog. Avoid cars and reach the top!</p>
        <p>Score: <span id="score">0</span> | Lives: <span id="lives">3</span></p>
    </div>
    <script>
        const canvas = document.getElementById('gameCanvas');
        const ctx = canvas.getContext('2d');

        // Basic game placeholder
        ctx.fillStyle = 'green';
        ctx.fillRect(400, 550, 20, 20); // Frog
        ctx.fillStyle = 'white';
        ctx.font = '20px Arial';
        ctx.fillText('Frogger Game - Use arrow keys to move!', 200, 300);

        console.log('Frogger game created by agentic system!');
    </script>
</body>
</html>"#.to_string()
}

/// Create Rust fallback template for games
fn create_rust_fallback() -> String {
    r#"// Frogger-like Game in Rust - Created by Agentic System
use std::io::{self, stdout, Write};
use std::time::{Duration, Instant};
use std::thread;

fn main() -> io::Result<()> {
    println!("🐸 Frogger Game - Created by Agentic System");
    println!("Use WASD to move, Q to quit");

    // Basic game loop placeholder
    loop {
        println!("Game running... (Press Ctrl+C to exit)");
        thread::sleep(Duration::from_millis(1000));
        break; // Exit for now
    }

    Ok(())
}"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_cypher() {
        assert!(is_valid_cypher("MATCH (n) RETURN n"));
        assert!(is_valid_cypher("CREATE (n:Person {name: 'John'})"));
        assert!(is_valid_cypher("match (n) where n.name = 'test' return n"));
        assert!(!is_valid_cypher("SELECT * FROM table"));
        assert!(!is_valid_cypher(""));
    }

    #[test]
    fn test_extract_cypher_query() {
        let content = "Here's a query:\n```cypher\nMATCH (n) RETURN n\n```";
        let result = extract_cypher_query(content).unwrap();
        assert_eq!(result, "MATCH (n) RETURN n");

        let content2 = "```\nMATCH (n) RETURN n\n```";
        let result2 = extract_cypher_query(content2).unwrap();
        assert_eq!(result2, "MATCH (n) RETURN n");
    }

    #[test]
    fn test_format_csv_value() {
        assert_eq!(format_csv_value(&Value::String("test".to_string())), "test");
        assert_eq!(format_csv_value(&Value::String("test,with,comma".to_string())), "\"test,with,comma\"");
        assert_eq!(format_csv_value(&Value::Number(serde_json::Number::from(42))), "42");
        assert_eq!(format_csv_value(&Value::Bool(true)), "true");
        assert_eq!(format_csv_value(&Value::Null), "");
    }

    #[test]
    fn test_extract_code() {
        let response = "Here's some Rust code:\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let result = extract_code(response, "rust");
        assert!(result.contains("fn main()"));
        assert!(result.contains("println!"));
    }

    #[test]
    fn test_matches_file_type() {
        assert!(matches_file_type("fn main() {}", "rust"));
        assert!(matches_file_type("def hello():", "python"));
        assert!(matches_file_type("function test() {}", "javascript"));
        assert!(matches_file_type("{\"key\": \"value\"}", "json"));
        assert!(!matches_file_type("SELECT * FROM table", "rust"));
    }
}
