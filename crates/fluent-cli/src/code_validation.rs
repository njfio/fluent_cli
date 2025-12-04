//! Semantic validation for generated code
//!
//! This module provides validation functionality for generated code across
//! multiple programming languages, checking syntax markers, requirements,
//! and code quality.

use tracing::debug;

/// Result of semantic validation for generated code
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the code passes all validation checks
    pub valid: bool,
    /// Quality score from 0.0 (worst) to 1.0 (best)
    pub score: f32,
    /// List of validation issues found
    pub issues: Vec<String>,
    /// Suggestions for improvement
    pub suggestions: Vec<String>,
}

impl ValidationResult {
    /// Create a new validation result
    pub fn new(valid: bool, score: f32) -> Self {
        Self {
            valid,
            score,
            issues: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Add an issue to the validation result
    pub fn add_issue(&mut self, issue: String) {
        self.issues.push(issue);
    }

    /// Add a suggestion to the validation result
    pub fn add_suggestion(&mut self, suggestion: String) {
        self.suggestions.push(suggestion);
    }

    /// Calculate final score based on checks passed
    pub fn calculate_score(checks_passed: usize, total_checks: usize) -> f32 {
        if total_checks == 0 {
            return 0.0;
        }
        (checks_passed as f32) / (total_checks as f32)
    }
}

/// Syntax validation check result
#[derive(Debug, Clone)]
struct SyntaxCheck {
    passed: bool,
    message: String,
    suggestion: Option<String>,
}

impl SyntaxCheck {
    fn passed(message: String) -> Self {
        Self {
            passed: true,
            message,
            suggestion: None,
        }
    }

    fn failed(message: String, suggestion: String) -> Self {
        Self {
            passed: false,
            message,
            suggestion: Some(suggestion),
        }
    }
}

/// Validate generated code with semantic checks for language syntax and requirements
///
/// # Arguments
///
/// * `code` - The generated code to validate
/// * `language` - The programming language (rust, python, javascript, lua, html)
/// * `requirements` - Slice of requirement strings to check for in the code
///
/// # Returns
///
/// A `ValidationResult` containing validation status, score, issues, and suggestions
///
/// # Example
///
/// ```
/// use fluent_cli::code_validation::validate_generated_code;
///
/// let code = "fn main() { println!(\"Hello\"); }";
/// let result = validate_generated_code(code, "rust", &["main", "println"]);
/// assert!(result.valid);
/// assert!(result.score > 0.5);
/// ```
pub fn validate_generated_code(
    code: &str,
    language: &str,
    requirements: &[&str],
) -> ValidationResult {
    let code_lower = code.to_lowercase();
    let mut checks_passed = 0;
    let mut total_checks = 0;
    let mut result = ValidationResult::new(true, 0.0);

    // Minimum size check
    total_checks += 1;
    let min_size = match language {
        "html" => 500,
        "rust" => 100,
        "python" => 50,
        "javascript" => 50,
        "lua" => 50,
        _ => 50,
    };

    if code.len() < min_size {
        result.add_issue(format!(
            "Code is too short ({} chars, minimum {})",
            code.len(),
            min_size
        ));
        result.add_suggestion(format!(
            "Add more implementation details (minimum {} characters expected)",
            min_size
        ));
        result.valid = false;
    } else {
        checks_passed += 1;
    }

    // Language-specific syntax checks
    let syntax_checks = match language {
        "rust" => validate_rust_syntax(&code_lower),
        "python" => validate_python_syntax(&code_lower),
        "javascript" => validate_javascript_syntax(&code_lower),
        "lua" => validate_lua_syntax(&code_lower),
        "html" => validate_html_syntax(&code_lower),
        _ => {
            result.add_issue(format!("Unknown language: {}", language));
            result.add_suggestion(
                "Use a supported language: rust, python, javascript, lua, html".to_string(),
            );
            return result;
        }
    };

    total_checks += syntax_checks.len();
    for check in &syntax_checks {
        if check.passed {
            checks_passed += 1;
        } else {
            result.add_issue(check.message.clone());
            if let Some(suggestion) = &check.suggestion {
                result.add_suggestion(suggestion.clone());
            }
            result.valid = false;
        }
    }

    // Requirements checking
    let req_results = validate_requirements(&code_lower, requirements);
    total_checks += req_results.len();
    for check in &req_results {
        if check.passed {
            checks_passed += 1;
        } else {
            result.add_issue(check.message.clone());
            if let Some(suggestion) = &check.suggestion {
                result.add_suggestion(suggestion.clone());
            }
        }
    }

    // Calculate final score
    result.score = ValidationResult::calculate_score(checks_passed, total_checks);

    // Determine validity: must pass at least 70% of checks
    if result.score < 0.7 {
        result.valid = false;
        result.add_suggestion(format!(
            "Score is {:.1}%. Aim for at least 70% to pass validation",
            result.score * 100.0
        ));
    }

    debug!(
        "validation.result language='{}' valid={} score={:.2} checks_passed={}/{}",
        language, result.valid, result.score, checks_passed, total_checks
    );

    result
}

/// Validate Rust syntax markers
fn validate_rust_syntax(code_lower: &str) -> Vec<SyntaxCheck> {
    let mut checks = Vec::new();

    // Check for fn main() or fn keyword
    if code_lower.contains("fn main()") || code_lower.contains("fn main(") {
        checks.push(SyntaxCheck::passed("Has main function".to_string()));
    } else if code_lower.contains("fn ") {
        checks.push(SyntaxCheck::passed("Has function definitions".to_string()));
    } else {
        checks.push(SyntaxCheck::failed(
            "No function definitions found".to_string(),
            "Add at least one function with 'fn' keyword".to_string(),
        ));
    }

    // Check for proper braces
    let open_braces = code_lower.matches('{').count();
    let close_braces = code_lower.matches('}').count();
    if open_braces == close_braces && open_braces > 0 {
        checks.push(SyntaxCheck::passed("Balanced braces".to_string()));
    } else {
        checks.push(SyntaxCheck::failed(
            format!(
                "Unbalanced braces: {} open, {} close",
                open_braces, close_braces
            ),
            "Ensure all opening braces have matching closing braces".to_string(),
        ));
    }

    // Check for common Rust keywords
    if code_lower.contains("let ") || code_lower.contains("mut ") {
        checks.push(SyntaxCheck::passed(
            "Uses variable declarations".to_string(),
        ));
    }

    checks
}

/// Validate Python syntax markers
fn validate_python_syntax(code_lower: &str) -> Vec<SyntaxCheck> {
    let mut checks = Vec::new();

    // Check for def or class
    if code_lower.contains("def ") {
        checks.push(SyntaxCheck::passed("Has function definitions".to_string()));
    } else if code_lower.contains("class ") {
        checks.push(SyntaxCheck::passed("Has class definitions".to_string()));
    } else {
        checks.push(SyntaxCheck::failed(
            "No function or class definitions found".to_string(),
            "Add at least one function with 'def' or class with 'class'".to_string(),
        ));
    }

    // Check for proper indentation markers (multiple spaces or tabs at line start)
    let has_indentation = code_lower.lines().any(|line| {
        line.starts_with("    ") || line.starts_with("\t") || line.starts_with("        ")
    });
    if has_indentation {
        checks.push(SyntaxCheck::passed(
            "Has proper indentation structure".to_string(),
        ));
    } else {
        checks.push(SyntaxCheck::failed(
            "No indentation found".to_string(),
            "Python requires proper indentation for code blocks".to_string(),
        ));
    }

    // Check for common Python patterns
    if code_lower.contains("import ") || code_lower.contains("from ") {
        checks.push(SyntaxCheck::passed("Has import statements".to_string()));
    }

    checks
}

/// Validate JavaScript syntax markers
fn validate_javascript_syntax(code_lower: &str) -> Vec<SyntaxCheck> {
    let mut checks = Vec::new();

    // Check for function declarations
    if code_lower.contains("function ")
        || code_lower.contains("const ")
        || code_lower.contains("let ")
        || code_lower.contains("var ")
    {
        checks.push(SyntaxCheck::passed(
            "Has function or variable declarations".to_string(),
        ));
    } else {
        checks.push(SyntaxCheck::failed(
            "No function or variable declarations found".to_string(),
            "Add functions with 'function' keyword or variables with 'const'/'let'/'var'"
                .to_string(),
        ));
    }

    // Check for proper braces
    let open_braces = code_lower.matches('{').count();
    let close_braces = code_lower.matches('}').count();
    if open_braces == close_braces && open_braces > 0 {
        checks.push(SyntaxCheck::passed("Balanced braces".to_string()));
    } else if open_braces > 0 || close_braces > 0 {
        checks.push(SyntaxCheck::failed(
            format!(
                "Unbalanced braces: {} open, {} close",
                open_braces, close_braces
            ),
            "Ensure all opening braces have matching closing braces".to_string(),
        ));
    }

    // Check for semicolons or arrow functions (common JS patterns)
    if code_lower.contains(';') || code_lower.contains("=>") {
        checks.push(SyntaxCheck::passed(
            "Has JavaScript syntax markers".to_string(),
        ));
    }

    checks
}

/// Validate Lua syntax markers
fn validate_lua_syntax(code_lower: &str) -> Vec<SyntaxCheck> {
    let mut checks = Vec::new();

    // Check for function or local declarations
    if code_lower.contains("function ") || code_lower.contains("local ") {
        checks.push(SyntaxCheck::passed(
            "Has function or local declarations".to_string(),
        ));
    } else {
        checks.push(SyntaxCheck::failed(
            "No function or variable declarations found".to_string(),
            "Add functions with 'function' keyword or variables with 'local'".to_string(),
        ));
    }

    // Check for Love2D callbacks (if it's a game)
    if code_lower.contains("love.load")
        || code_lower.contains("love.draw")
        || code_lower.contains("love.update")
    {
        checks.push(SyntaxCheck::passed("Has Love2D callbacks".to_string()));
    }

    // Check for proper end statements
    let function_count = code_lower.matches("function ").count();
    let end_count = code_lower.matches(" end").count() + code_lower.matches("\nend").count();
    if function_count > 0 && end_count >= function_count {
        checks.push(SyntaxCheck::passed("Has proper end statements".to_string()));
    } else if function_count > 0 {
        checks.push(SyntaxCheck::failed(
            format!(
                "Missing end statements: {} functions, {} ends",
                function_count, end_count
            ),
            "Every Lua function needs an 'end' statement".to_string(),
        ));
    }

    checks
}

/// Validate HTML syntax markers
fn validate_html_syntax(code_lower: &str) -> Vec<SyntaxCheck> {
    let mut checks = Vec::new();

    // Check for basic HTML structure
    if code_lower.contains("<html") || code_lower.contains("<!doctype html") {
        checks.push(SyntaxCheck::passed(
            "Has HTML document structure".to_string(),
        ));
    } else {
        checks.push(SyntaxCheck::failed(
            "Missing HTML document structure".to_string(),
            "Add <!DOCTYPE html> and <html> tags".to_string(),
        ));
    }

    // Check for head and body tags
    if code_lower.contains("<head") && code_lower.contains("<body") {
        checks.push(SyntaxCheck::passed("Has head and body tags".to_string()));
    } else {
        checks.push(SyntaxCheck::failed(
            "Missing head or body tags".to_string(),
            "HTML documents should have <head> and <body> sections".to_string(),
        ));
    }

    // Check for script or style tags (for interactive content)
    if code_lower.contains("<script") || code_lower.contains("<style") {
        checks.push(SyntaxCheck::passed(
            "Has embedded scripts or styles".to_string(),
        ));
    }

    // Check for canvas (common in game development)
    if code_lower.contains("<canvas") {
        checks.push(SyntaxCheck::passed("Has canvas element".to_string()));
    }

    checks
}

/// Validate that code contains required keywords/patterns
fn validate_requirements(code_lower: &str, requirements: &[&str]) -> Vec<SyntaxCheck> {
    let mut checks = Vec::new();

    for requirement in requirements {
        let req_lower = requirement.to_lowercase();
        // Check for the requirement or related variations
        if code_lower.contains(&req_lower) {
            checks.push(SyntaxCheck::passed(format!(
                "Contains required keyword: {}",
                requirement
            )));
        } else {
            checks.push(SyntaxCheck::failed(
                format!("Missing required keyword: {}", requirement),
                format!("Add implementation related to '{}'", requirement),
            ));
        }
    }

    checks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_rust_code() {
        let code = r#"
            fn main() {
                let x = 5;
                println!("Hello, world!");
            }
        "#;
        let result = validate_generated_code(code, "rust", &["main", "println"]);
        assert!(result.valid);
        assert!(result.score > 0.8);
    }

    #[test]
    fn test_validate_python_code() {
        let code = r#"
def greet(name):
    print(f"Hello, {name}!")

if __name__ == "__main__":
    greet("World")
        "#;
        let result = validate_generated_code(code, "python", &["def", "print"]);
        assert!(result.valid);
        assert!(result.score > 0.7);
    }

    #[test]
    fn test_validate_javascript_code() {
        let code = r#"
            function greet(name) {
                console.log(`Hello, ${name}!`);
            }
            const message = "test";
        "#;
        let result = validate_generated_code(code, "javascript", &["function", "console"]);
        assert!(result.valid);
        assert!(result.score > 0.7);
    }

    #[test]
    fn test_validate_lua_code() {
        let code = r#"
            function love.load()
                player = {x = 100, y = 100}
            end

            function love.draw()
                love.graphics.circle("fill", player.x, player.y, 50)
            end
        "#;
        let result = validate_generated_code(code, "lua", &["love", "player"]);
        assert!(result.valid);
        assert!(result.score > 0.7);
    }

    #[test]
    fn test_validate_html_code() {
        let code = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Test Game</title>
    <style>
        body {
            background: #333;
            margin: 0;
            padding: 0;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
        }
        canvas {
            border: 2px solid white;
        }
    </style>
</head>
<body>
    <canvas id="game" width="800" height="600"></canvas>
    <script>
        const canvas = document.getElementById('game');
        const ctx = canvas.getContext('2d');

        function gameLoop() {
            ctx.fillStyle = '#000';
            ctx.fillRect(0, 0, canvas.width, canvas.height);
            requestAnimationFrame(gameLoop);
        }

        gameLoop();
    </script>
</body>
</html>
        "#;
        let result = validate_generated_code(code, "html", &["canvas", "script"]);
        assert!(result.valid, "HTML validation failed: {:?}", result.issues);
        assert!(
            result.score > 0.8,
            "HTML validation score too low: {:.2}",
            result.score
        );
    }

    #[test]
    fn test_invalid_code_too_short() {
        let code = "fn main() {}";
        let result = validate_generated_code(code, "rust", &[]);
        assert!(!result.valid);
        assert!(result.issues.iter().any(|i| i.contains("too short")));
    }

    #[test]
    fn test_missing_requirements() {
        let code = r#"
            fn main() {
                let x = 5;
                println!("Hello");
            }
        "#;
        let result = validate_generated_code(code, "rust", &["database", "connection"]);
        assert!(!result.valid);
        assert!(result
            .issues
            .iter()
            .any(|i| i.contains("Missing required keyword")));
    }
}
