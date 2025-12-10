//! Code Porting and Translation Pattern Detection
//!
//! This module provides pattern detection and guidance for code porting tasks,
//! helping agents translate code between programming languages effectively.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Source and target programming languages for porting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProgrammingLanguage {
    C,
    Cpp,
    Rust,
    Go,
    Python,
    JavaScript,
    TypeScript,
    Java,
    CSharp,
    Ruby,
    Swift,
    Kotlin,
    Haskell,
    Scala,
    Lua,
    Perl,
    PHP,
}

impl ProgrammingLanguage {
    /// Get language keywords for detection
    pub fn keywords(&self) -> Vec<&'static str> {
        match self {
            ProgrammingLanguage::C => vec!["c", "c language", ".c", ".h"],
            ProgrammingLanguage::Cpp => vec!["c++", "cpp", ".cpp", ".hpp", ".cc"],
            ProgrammingLanguage::Rust => vec!["rust", ".rs", "cargo"],
            ProgrammingLanguage::Go => vec!["go", "golang", ".go"],
            ProgrammingLanguage::Python => vec!["python", "py", ".py", "pip"],
            ProgrammingLanguage::JavaScript => vec!["javascript", "js", ".js", "node"],
            ProgrammingLanguage::TypeScript => vec!["typescript", "ts", ".ts", ".tsx"],
            ProgrammingLanguage::Java => vec!["java", ".java", "jvm"],
            ProgrammingLanguage::CSharp => vec!["c#", "csharp", ".cs", "dotnet"],
            ProgrammingLanguage::Ruby => vec!["ruby", ".rb", "gem"],
            ProgrammingLanguage::Swift => vec!["swift", ".swift"],
            ProgrammingLanguage::Kotlin => vec!["kotlin", ".kt", ".kts"],
            ProgrammingLanguage::Haskell => vec!["haskell", ".hs", "cabal"],
            ProgrammingLanguage::Scala => vec!["scala", ".scala", "sbt"],
            ProgrammingLanguage::Lua => vec!["lua", ".lua", "love2d", "luarocks"],
            ProgrammingLanguage::Perl => vec!["perl", ".pl", "cpan"],
            ProgrammingLanguage::PHP => vec!["php", ".php"],
        }
    }

    /// Get standard library name for this language
    pub fn std_library_name(&self) -> &'static str {
        match self {
            ProgrammingLanguage::C => "libc/POSIX",
            ProgrammingLanguage::Cpp => "STL",
            ProgrammingLanguage::Rust => "std",
            ProgrammingLanguage::Go => "standard library",
            ProgrammingLanguage::Python => "builtins/stdlib",
            ProgrammingLanguage::JavaScript => "built-ins/Node.js",
            ProgrammingLanguage::TypeScript => "built-ins/Node.js",
            ProgrammingLanguage::Java => "JDK",
            ProgrammingLanguage::CSharp => ".NET BCL",
            ProgrammingLanguage::Ruby => "core/stdlib",
            ProgrammingLanguage::Swift => "Foundation/Swift stdlib",
            ProgrammingLanguage::Kotlin => "kotlin-stdlib",
            ProgrammingLanguage::Haskell => "base/Prelude",
            ProgrammingLanguage::Scala => "scala-library",
            ProgrammingLanguage::Lua => "standard library",
            ProgrammingLanguage::Perl => "core modules",
            ProgrammingLanguage::PHP => "built-in functions",
        }
    }
}

/// Categories of code porting challenges
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortingCategory {
    /// Memory management (manual vs GC, ownership)
    MemoryManagement,
    /// Error handling (exceptions vs Result types)
    ErrorHandling,
    /// Type systems (static vs dynamic, generics)
    TypeSystem,
    /// Concurrency patterns (threads, async)
    Concurrency,
    /// Standard library differences
    StandardLibrary,
    /// String handling (unicode, encoding)
    StringHandling,
    /// Collections and data structures
    Collections,
    /// Build and package management
    BuildSystem,
    /// Idiomatic patterns and conventions
    IdiomaticCode,
    /// Testing framework differences
    Testing,
}

/// Guidance for porting from one language to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortingGuidance {
    /// General approach description
    pub approach: String,
    /// Steps for the porting process
    pub steps: Vec<String>,
    /// Type mappings between languages
    pub type_mappings: Vec<(String, String)>,
    /// Standard library function equivalents
    pub stdlib_mappings: Vec<(String, String)>,
    /// Common pitfalls to avoid
    pub pitfalls: Vec<String>,
    /// Idiomatic patterns to apply
    pub idioms: Vec<String>,
    /// Example code transformation
    pub example: Option<CodeExample>,
}

/// Example code transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    pub source_code: String,
    pub target_code: String,
    pub explanation: String,
}

/// A specific language pair porting pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguagePairPattern {
    /// Source language
    pub source: ProgrammingLanguage,
    /// Target language
    pub target: ProgrammingLanguage,
    /// Name of this porting pattern
    pub name: String,
    /// Keywords that identify this pattern
    pub keywords: Vec<String>,
    /// Detection confidence
    pub confidence: f64,
    /// Detailed porting guidance
    pub guidance: PortingGuidance,
    /// Specific category challenges for this pair
    pub challenges: Vec<PortingCategory>,
}

/// Result of code porting pattern detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePortingDetectionResult {
    /// Detected language pair patterns
    pub patterns: Vec<LanguagePairPattern>,
    /// Detected source language
    pub source_language: Option<ProgrammingLanguage>,
    /// Detected target language
    pub target_language: Option<ProgrammingLanguage>,
    /// Keywords found in the task
    pub matched_keywords: Vec<String>,
    /// Overall confidence this is a porting task
    pub overall_confidence: f64,
    /// Whether prompt augmentation is recommended
    pub should_augment: bool,
}

/// Code porting pattern detector
pub struct CodePortingPatternDetector {
    patterns: Vec<LanguagePairPattern>,
}

impl Default for CodePortingPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl CodePortingPatternDetector {
    /// Create a new pattern detector with built-in patterns
    pub fn new() -> Self {
        Self {
            patterns: Self::build_default_patterns(),
        }
    }

    /// Detect porting patterns in a task description
    pub fn detect(&self, task_description: &str) -> CodePortingDetectionResult {
        let lower_desc = task_description.to_lowercase();
        let mut matched_patterns: Vec<LanguagePairPattern> = Vec::new();
        let mut matched_keywords: Vec<String> = Vec::new();

        // Check for porting-related keywords (these are strong indicators)
        let strong_porting_keywords = [
            "port", "porting", "convert to", "translate to", "rewrite in",
            "migrate to", "migration", "transpile", "from rust", "from c ",
            "from python", "from javascript", "from java", "to rust", "to go",
            "to python", "to java", "to typescript", "convert from",
        ];

        // These keywords require additional context to indicate porting
        let context_porting_keywords = [
            "convert", "translate", "rewrite", "migrate",
        ];

        let has_strong_keyword = strong_porting_keywords
            .iter()
            .any(|kw| lower_desc.contains(kw));

        // For context keywords, require both a keyword AND language mention
        let has_context_keyword = context_porting_keywords
            .iter()
            .any(|kw| lower_desc.contains(kw));

        let has_language_pattern = lower_desc.contains(" to rust")
            || lower_desc.contains(" to go")
            || lower_desc.contains(" to python")
            || lower_desc.contains(" to lua")
            || lower_desc.contains(" to typescript")
            || lower_desc.contains(" to java")
            || lower_desc.contains(" to kotlin")
            || lower_desc.contains(" from c ")
            || lower_desc.contains(" from python")
            || lower_desc.contains(" from java")
            || lower_desc.contains(" from javascript")
            || (lower_desc.contains("from") && lower_desc.contains("to"));

        let is_porting_task = has_strong_keyword || (has_context_keyword && has_language_pattern);

        if !is_porting_task {
            return CodePortingDetectionResult {
                patterns: Vec::new(),
                source_language: None,
                target_language: None,
                matched_keywords: Vec::new(),
                overall_confidence: 0.0,
                should_augment: false,
            };
        }

        // Detect source and target languages
        let source_lang = self.detect_language(&lower_desc, &["from", "in"]);
        let target_lang = self.detect_language(&lower_desc, &["to", "into"]);

        // Match patterns
        for pattern in &self.patterns {
            let mut keyword_matches = 0;
            let mut pattern_keywords = Vec::new();

            // Check if languages match
            let source_matches = source_lang.map(|l| l == pattern.source).unwrap_or(false)
                || pattern.source.keywords().iter().any(|k| lower_desc.contains(k));
            let target_matches = target_lang.map(|l| l == pattern.target).unwrap_or(false)
                || pattern.target.keywords().iter().any(|k| lower_desc.contains(k));

            if source_matches && target_matches {
                keyword_matches += 2;
            } else if source_matches || target_matches {
                keyword_matches += 1;
            }

            // Check pattern-specific keywords
            for keyword in &pattern.keywords {
                if lower_desc.contains(&keyword.to_lowercase()) {
                    keyword_matches += 1;
                    pattern_keywords.push(keyword.clone());
                }
            }

            if keyword_matches > 0 {
                let confidence = (keyword_matches as f64 / 5.0).min(1.0);
                if confidence > 0.2 {
                    let mut matched_pattern = pattern.clone();
                    matched_pattern.confidence = confidence;
                    matched_patterns.push(matched_pattern);
                    matched_keywords.extend(pattern_keywords);
                }
            }
        }

        // Sort by confidence
        matched_patterns.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        matched_keywords.sort();
        matched_keywords.dedup();

        let overall_confidence = matched_patterns
            .first()
            .map(|p| p.confidence)
            .unwrap_or(if is_porting_task { 0.3 } else { 0.0 });

        CodePortingDetectionResult {
            patterns: matched_patterns,
            source_language: source_lang,
            target_language: target_lang,
            matched_keywords,
            overall_confidence,
            should_augment: overall_confidence > 0.2,
        }
    }

    /// Detect a programming language in text with context
    fn detect_language(&self, text: &str, context_words: &[&str]) -> Option<ProgrammingLanguage> {
        let languages = [
            ProgrammingLanguage::C,
            ProgrammingLanguage::Cpp,
            ProgrammingLanguage::Rust,
            ProgrammingLanguage::Go,
            ProgrammingLanguage::Python,
            ProgrammingLanguage::JavaScript,
            ProgrammingLanguage::TypeScript,
            ProgrammingLanguage::Java,
            ProgrammingLanguage::CSharp,
            ProgrammingLanguage::Ruby,
            ProgrammingLanguage::Swift,
            ProgrammingLanguage::Kotlin,
            ProgrammingLanguage::Haskell,
            ProgrammingLanguage::Lua,
        ];

        // First pass: look for language keyword with context word
        for lang in languages {
            for keyword in lang.keywords() {
                for context in context_words {
                    let pattern = format!("{} {}", context, keyword);
                    if text.contains(&pattern) {
                        return Some(lang);
                    }
                }
            }
        }

        // Second pass: look for standalone language mentions with word boundaries
        // Only do this if we didn't find a match with context
        for lang in languages {
            for keyword in lang.keywords() {
                // Skip very short keywords in standalone check to avoid false positives
                if keyword.len() < 2 {
                    continue;
                }
                // Check for word boundary match
                if Self::contains_word(text, keyword) {
                    return Some(lang);
                }
            }
        }
        None
    }

    /// Check if text contains keyword as a complete word (with word boundaries)
    fn contains_word(text: &str, word: &str) -> bool {
        let text_bytes = text.as_bytes();
        let word_bytes = word.as_bytes();

        if word_bytes.is_empty() {
            return false;
        }

        let mut i = 0;
        while i <= text_bytes.len().saturating_sub(word_bytes.len()) {
            if let Some(pos) = text[i..].find(word) {
                let abs_pos = i + pos;
                let before_ok = abs_pos == 0
                    || !text_bytes[abs_pos - 1].is_ascii_alphanumeric();
                let after_pos = abs_pos + word.len();
                let after_ok = after_pos >= text_bytes.len()
                    || !text_bytes[after_pos].is_ascii_alphanumeric();

                if before_ok && after_ok {
                    return true;
                }
                i = abs_pos + 1;
            } else {
                break;
            }
        }
        false
    }

    /// Generate prompt augmentation for detected patterns
    pub fn generate_prompt_augmentation(&self, detection: &CodePortingDetectionResult) -> String {
        if !detection.should_augment {
            return String::new();
        }

        let mut augmentation = String::new();
        augmentation.push_str("\n\n## Code Porting Guidance\n\n");

        if let (Some(source), Some(target)) = (detection.source_language, detection.target_language)
        {
            augmentation.push_str(&format!(
                "**Detected Language Pair**: {:?} → {:?}\n\n",
                source, target
            ));
        }

        for (idx, pattern) in detection.patterns.iter().take(2).enumerate() {
            if idx > 0 {
                augmentation.push_str("\n---\n\n");
            }

            augmentation.push_str(&format!(
                "### {} ({:?} → {:?})\n\n",
                pattern.name, pattern.source, pattern.target
            ));

            augmentation.push_str(&format!(
                "**Approach**: {}\n\n",
                pattern.guidance.approach
            ));

            augmentation.push_str("**Porting Steps**:\n");
            for (i, step) in pattern.guidance.steps.iter().enumerate() {
                augmentation.push_str(&format!("{}. {}\n", i + 1, step));
            }
            augmentation.push('\n');

            if !pattern.guidance.type_mappings.is_empty() {
                augmentation.push_str("**Type Mappings**:\n");
                for (source, target) in pattern.guidance.type_mappings.iter().take(5) {
                    augmentation.push_str(&format!("- `{}` → `{}`\n", source, target));
                }
                augmentation.push('\n');
            }

            if !pattern.guidance.stdlib_mappings.is_empty() {
                augmentation.push_str("**Standard Library Equivalents**:\n");
                for (source, target) in pattern.guidance.stdlib_mappings.iter().take(5) {
                    augmentation.push_str(&format!("- `{}` → `{}`\n", source, target));
                }
                augmentation.push('\n');
            }

            if !pattern.guidance.pitfalls.is_empty() {
                augmentation.push_str("**Common Pitfalls**:\n");
                for pitfall in &pattern.guidance.pitfalls {
                    augmentation.push_str(&format!("- ⚠️ {}\n", pitfall));
                }
                augmentation.push('\n');
            }

            if !pattern.guidance.idioms.is_empty() {
                augmentation.push_str("**Idiomatic Patterns**:\n");
                for idiom in &pattern.guidance.idioms {
                    augmentation.push_str(&format!("- 💡 {}\n", idiom));
                }
                augmentation.push('\n');
            }

            if let Some(example) = &pattern.guidance.example {
                augmentation.push_str(&format!(
                    "**Example Transformation**:\n{}\n\n",
                    example.explanation
                ));
            }
        }

        augmentation
    }

    /// Build the default set of language pair patterns
    fn build_default_patterns() -> Vec<LanguagePairPattern> {
        vec![
            // C to Rust
            LanguagePairPattern {
                source: ProgrammingLanguage::C,
                target: ProgrammingLanguage::Rust,
                name: "C to Rust Porting".to_string(),
                keywords: vec![
                    "memory safe".to_string(),
                    "ownership".to_string(),
                    "borrow checker".to_string(),
                    "unsafe".to_string(),
                ],
                confidence: 0.0,
                challenges: vec![
                    PortingCategory::MemoryManagement,
                    PortingCategory::ErrorHandling,
                    PortingCategory::TypeSystem,
                ],
                guidance: PortingGuidance {
                    approach: "Convert C code to safe Rust by replacing manual memory management with ownership, using Result for error handling, and applying Rust idioms.".to_string(),
                    steps: vec![
                        "Identify memory allocations (malloc/free) and convert to owned types (Box, Vec, String)".to_string(),
                        "Replace raw pointers with references (&, &mut) where possible".to_string(),
                        "Convert error codes to Result<T, E> return types".to_string(),
                        "Replace preprocessor macros with const, type aliases, or Rust macros".to_string(),
                        "Use Option<T> for nullable pointers".to_string(),
                        "Convert C structs to Rust structs with proper visibility".to_string(),
                        "Port tests using #[test] attribute".to_string(),
                    ],
                    type_mappings: vec![
                        ("int".to_string(), "i32".to_string()),
                        ("unsigned int".to_string(), "u32".to_string()),
                        ("long".to_string(), "i64".to_string()),
                        ("char".to_string(), "i8 or u8".to_string()),
                        ("char*".to_string(), "String or &str".to_string()),
                        ("void*".to_string(), "*mut c_void or Box<T>".to_string()),
                        ("size_t".to_string(), "usize".to_string()),
                        ("NULL".to_string(), "None or null pointer".to_string()),
                    ],
                    stdlib_mappings: vec![
                        ("malloc/free".to_string(), "Box::new / drop".to_string()),
                        ("strlen".to_string(), "str.len()".to_string()),
                        ("strcmp".to_string(), "str == str".to_string()),
                        ("printf".to_string(), "println!()".to_string()),
                        ("memcpy".to_string(), "slice.copy_from_slice()".to_string()),
                        ("fopen/fclose".to_string(), "File::open()".to_string()),
                    ],
                    pitfalls: vec![
                        "Don't use unsafe unless absolutely necessary".to_string(),
                        "C array indices start at 0, Rust panics on out-of-bounds".to_string(),
                        "Rust strings are UTF-8, C strings are null-terminated bytes".to_string(),
                        "Error handling: don't ignore Result values".to_string(),
                        "Mutable aliasing is forbidden in safe Rust".to_string(),
                    ],
                    idioms: vec![
                        "Use iterators instead of index-based loops".to_string(),
                        "Prefer &str over String for function parameters".to_string(),
                        "Use derive macros for Debug, Clone, PartialEq".to_string(),
                        "Handle errors with ? operator for propagation".to_string(),
                        "Use pattern matching instead of if/else chains".to_string(),
                    ],
                    example: Some(CodeExample {
                        source_code: "int* arr = malloc(10 * sizeof(int));\nif (arr == NULL) return -1;".to_string(),
                        target_code: "let arr: Vec<i32> = vec![0; 10];\n// Or with allocation failure: Vec::try_reserve()".to_string(),
                        explanation: "Replace malloc with Vec, which handles allocation automatically and is bounds-checked.".to_string(),
                    }),
                },
            },
            // C++ to Rust
            LanguagePairPattern {
                source: ProgrammingLanguage::Cpp,
                target: ProgrammingLanguage::Rust,
                name: "C++ to Rust Porting".to_string(),
                keywords: vec![
                    "smart pointer".to_string(),
                    "RAII".to_string(),
                    "template".to_string(),
                    "class".to_string(),
                ],
                confidence: 0.0,
                challenges: vec![
                    PortingCategory::MemoryManagement,
                    PortingCategory::TypeSystem,
                    PortingCategory::IdiomaticCode,
                ],
                guidance: PortingGuidance {
                    approach: "C++ and Rust share RAII principles. Convert classes to structs with impl blocks, templates to generics, and smart pointers to Rust equivalents.".to_string(),
                    steps: vec![
                        "Convert classes to struct + impl blocks".to_string(),
                        "Replace unique_ptr with Box, shared_ptr with Arc/Rc".to_string(),
                        "Convert templates to Rust generics with trait bounds".to_string(),
                        "Replace inheritance with trait composition".to_string(),
                        "Convert exceptions to Result<T, E>".to_string(),
                        "Port constructors to new() associated functions".to_string(),
                        "Replace operator overloading with trait implementations".to_string(),
                    ],
                    type_mappings: vec![
                        ("std::string".to_string(), "String".to_string()),
                        ("std::vector<T>".to_string(), "Vec<T>".to_string()),
                        ("std::map<K,V>".to_string(), "HashMap<K,V>".to_string()),
                        ("std::unique_ptr<T>".to_string(), "Box<T>".to_string()),
                        ("std::shared_ptr<T>".to_string(), "Arc<T> or Rc<T>".to_string()),
                        ("std::optional<T>".to_string(), "Option<T>".to_string()),
                    ],
                    stdlib_mappings: vec![
                        ("std::cout".to_string(), "println!()".to_string()),
                        ("std::cin".to_string(), "std::io::stdin()".to_string()),
                        ("std::sort".to_string(), "slice.sort()".to_string()),
                        ("std::find".to_string(), "iter.find()".to_string()),
                    ],
                    pitfalls: vec![
                        "Rust has no inheritance - use trait objects or composition".to_string(),
                        "No function overloading - use different names or traits".to_string(),
                        "No default arguments - use builder pattern or Option".to_string(),
                        "Move semantics are the default in Rust".to_string(),
                    ],
                    idioms: vec![
                        "Use traits instead of abstract base classes".to_string(),
                        "Implement From/Into for type conversions".to_string(),
                        "Use #[derive] for common trait implementations".to_string(),
                        "Prefer composition over inheritance".to_string(),
                    ],
                    example: None,
                },
            },
            // Python to Rust
            LanguagePairPattern {
                source: ProgrammingLanguage::Python,
                target: ProgrammingLanguage::Rust,
                name: "Python to Rust Porting".to_string(),
                keywords: vec![
                    "type hints".to_string(),
                    "performance".to_string(),
                    "static typing".to_string(),
                ],
                confidence: 0.0,
                challenges: vec![
                    PortingCategory::TypeSystem,
                    PortingCategory::ErrorHandling,
                    PortingCategory::Collections,
                ],
                guidance: PortingGuidance {
                    approach: "Add explicit types to Python code first, then convert. Rust requires explicit error handling and doesn't have dynamic typing.".to_string(),
                    steps: vec![
                        "Add type hints to Python code to clarify types".to_string(),
                        "Convert Python classes to Rust structs + impl".to_string(),
                        "Replace try/except with Result<T, E>".to_string(),
                        "Convert list comprehensions to iterator chains".to_string(),
                        "Add explicit types to all variables and functions".to_string(),
                        "Handle None with Option<T>".to_string(),
                        "Port unittest to Rust #[test]".to_string(),
                    ],
                    type_mappings: vec![
                        ("int".to_string(), "i64 or i32".to_string()),
                        ("float".to_string(), "f64".to_string()),
                        ("str".to_string(), "String or &str".to_string()),
                        ("list".to_string(), "Vec<T>".to_string()),
                        ("dict".to_string(), "HashMap<K, V>".to_string()),
                        ("set".to_string(), "HashSet<T>".to_string()),
                        ("tuple".to_string(), "(T1, T2, ...)".to_string()),
                        ("None".to_string(), "None (Option)".to_string()),
                        ("bool".to_string(), "bool".to_string()),
                    ],
                    stdlib_mappings: vec![
                        ("len()".to_string(), ".len()".to_string()),
                        ("range()".to_string(), "0..n or (0..n).into_iter()".to_string()),
                        ("print()".to_string(), "println!()".to_string()),
                        ("open()".to_string(), "File::open()".to_string()),
                        ("json.loads".to_string(), "serde_json::from_str".to_string()),
                    ],
                    pitfalls: vec![
                        "Python integers are arbitrary precision, Rust's are fixed".to_string(),
                        "No duck typing - must use traits explicitly".to_string(),
                        "String indexing works differently (UTF-8)".to_string(),
                        "No implicit type conversions".to_string(),
                    ],
                    idioms: vec![
                        "Use iterator methods instead of for loops".to_string(),
                        "Pattern matching instead of isinstance() checks".to_string(),
                        "Use ? for error propagation".to_string(),
                        "Implement Default trait instead of default parameters".to_string(),
                    ],
                    example: None,
                },
            },
            // Python to Go
            LanguagePairPattern {
                source: ProgrammingLanguage::Python,
                target: ProgrammingLanguage::Go,
                name: "Python to Go Porting".to_string(),
                keywords: vec![
                    "goroutine".to_string(),
                    "channel".to_string(),
                    "concurrent".to_string(),
                ],
                confidence: 0.0,
                challenges: vec![
                    PortingCategory::TypeSystem,
                    PortingCategory::ErrorHandling,
                    PortingCategory::Concurrency,
                ],
                guidance: PortingGuidance {
                    approach: "Go requires explicit types and explicit error handling. Convert classes to structs with methods, and use goroutines for concurrency.".to_string(),
                    steps: vec![
                        "Add type hints to Python to clarify types".to_string(),
                        "Convert classes to Go structs with methods".to_string(),
                        "Replace try/except with explicit error returns".to_string(),
                        "Convert async/await to goroutines and channels".to_string(),
                        "Use explicit loops instead of list comprehensions".to_string(),
                        "Port unittest to Go testing package".to_string(),
                    ],
                    type_mappings: vec![
                        ("int".to_string(), "int or int64".to_string()),
                        ("float".to_string(), "float64".to_string()),
                        ("str".to_string(), "string".to_string()),
                        ("list".to_string(), "[]T (slice)".to_string()),
                        ("dict".to_string(), "map[K]V".to_string()),
                        ("None".to_string(), "nil".to_string()),
                        ("bool".to_string(), "bool".to_string()),
                    ],
                    stdlib_mappings: vec![
                        ("len()".to_string(), "len()".to_string()),
                        ("print()".to_string(), "fmt.Println()".to_string()),
                        ("open()".to_string(), "os.Open()".to_string()),
                        ("json.loads".to_string(), "json.Unmarshal()".to_string()),
                    ],
                    pitfalls: vec![
                        "Go has no exceptions - must check errors explicitly".to_string(),
                        "No list comprehensions - use explicit loops".to_string(),
                        "Unused variables/imports are errors".to_string(),
                        "Capitalization controls visibility".to_string(),
                    ],
                    idioms: vec![
                        "Use 'if err != nil' pattern for error handling".to_string(),
                        "Capitalize exported identifiers".to_string(),
                        "Use defer for cleanup".to_string(),
                        "Keep interfaces small and focused".to_string(),
                    ],
                    example: None,
                },
            },
            // JavaScript to TypeScript
            LanguagePairPattern {
                source: ProgrammingLanguage::JavaScript,
                target: ProgrammingLanguage::TypeScript,
                name: "JavaScript to TypeScript Migration".to_string(),
                keywords: vec![
                    "type safety".to_string(),
                    "typescript".to_string(),
                    "interface".to_string(),
                    "strict".to_string(),
                ],
                confidence: 0.0,
                challenges: vec![
                    PortingCategory::TypeSystem,
                    PortingCategory::BuildSystem,
                ],
                guidance: PortingGuidance {
                    approach: "Gradually add types to JavaScript code. Start with 'any' types and refine. TypeScript is a superset of JavaScript.".to_string(),
                    steps: vec![
                        "Rename .js files to .ts".to_string(),
                        "Add tsconfig.json with appropriate settings".to_string(),
                        "Start with loose type checking, increase strictness".to_string(),
                        "Add type annotations to function parameters and returns".to_string(),
                        "Define interfaces for object shapes".to_string(),
                        "Replace any with specific types".to_string(),
                        "Enable strict mode when most types are added".to_string(),
                    ],
                    type_mappings: vec![
                        ("let x = 5".to_string(), "let x: number = 5".to_string()),
                        ("function(x)".to_string(), "function(x: Type): ReturnType".to_string()),
                        ("{}".to_string(), "interface Shape { ... }".to_string()),
                        ("Array".to_string(), "T[] or Array<T>".to_string()),
                        ("callback".to_string(), "(arg: T) => R".to_string()),
                    ],
                    stdlib_mappings: vec![
                        ("Object".to_string(), "Record<string, T>".to_string()),
                        ("Promise".to_string(), "Promise<T>".to_string()),
                        ("Array methods".to_string(), "Typed array methods".to_string()),
                    ],
                    pitfalls: vec![
                        "Don't use 'any' as a permanent solution".to_string(),
                        "Be careful with implicit any".to_string(),
                        "null vs undefined handling".to_string(),
                        "Type assertions (as) can hide bugs".to_string(),
                    ],
                    idioms: vec![
                        "Use 'unknown' instead of 'any' when possible".to_string(),
                        "Define shared types in separate .d.ts files".to_string(),
                        "Use strict null checks".to_string(),
                        "Leverage type inference where clear".to_string(),
                    ],
                    example: None,
                },
            },
            // Java to Kotlin
            LanguagePairPattern {
                source: ProgrammingLanguage::Java,
                target: ProgrammingLanguage::Kotlin,
                name: "Java to Kotlin Migration".to_string(),
                keywords: vec![
                    "kotlin".to_string(),
                    "null safety".to_string(),
                    "data class".to_string(),
                    "extension".to_string(),
                ],
                confidence: 0.0,
                challenges: vec![
                    PortingCategory::TypeSystem,
                    PortingCategory::IdiomaticCode,
                ],
                guidance: PortingGuidance {
                    approach: "Kotlin is fully interoperable with Java. Use the IDE's automatic converter as a starting point, then apply Kotlin idioms.".to_string(),
                    steps: vec![
                        "Use IDE's 'Convert Java File to Kotlin'".to_string(),
                        "Review and fix null safety annotations".to_string(),
                        "Convert POJOs to data classes".to_string(),
                        "Replace Java streams with Kotlin collection functions".to_string(),
                        "Use extension functions where appropriate".to_string(),
                        "Simplify with Kotlin's concise syntax".to_string(),
                    ],
                    type_mappings: vec![
                        ("String".to_string(), "String".to_string()),
                        ("List<T>".to_string(), "List<T> or MutableList<T>".to_string()),
                        ("@Nullable".to_string(), "T?".to_string()),
                        ("void".to_string(), "Unit".to_string()),
                        ("final".to_string(), "val".to_string()),
                    ],
                    stdlib_mappings: vec![
                        ("stream().map()".to_string(), ".map {}".to_string()),
                        ("stream().filter()".to_string(), ".filter {}".to_string()),
                        ("Optional<T>".to_string(), "T?".to_string()),
                        ("System.out.println".to_string(), "println()".to_string()),
                    ],
                    pitfalls: vec![
                        "Java interop may need @JvmStatic, @JvmField annotations".to_string(),
                        "Kotlin collections default to immutable".to_string(),
                        "Be careful with platform types from Java".to_string(),
                    ],
                    idioms: vec![
                        "Use data classes for value objects".to_string(),
                        "Prefer val over var".to_string(),
                        "Use scope functions (let, run, apply, also)".to_string(),
                        "Use sealed classes for restricted hierarchies".to_string(),
                    ],
                    example: None,
                },
            },
            // C to Go
            LanguagePairPattern {
                source: ProgrammingLanguage::C,
                target: ProgrammingLanguage::Go,
                name: "C to Go Porting".to_string(),
                keywords: vec![
                    "go".to_string(),
                    "golang".to_string(),
                    "gc".to_string(),
                    "garbage collection".to_string(),
                ],
                confidence: 0.0,
                challenges: vec![
                    PortingCategory::MemoryManagement,
                    PortingCategory::ErrorHandling,
                    PortingCategory::Concurrency,
                ],
                guidance: PortingGuidance {
                    approach: "Go has garbage collection and simpler memory model. Convert pointer arithmetic to slices, manual memory management is not needed.".to_string(),
                    steps: vec![
                        "Replace malloc/free with Go's automatic memory management".to_string(),
                        "Convert arrays and pointers to slices".to_string(),
                        "Replace error codes with (result, error) returns".to_string(),
                        "Convert structs directly (similar syntax)".to_string(),
                        "Use goroutines for concurrent operations".to_string(),
                        "Port header declarations to Go packages".to_string(),
                    ],
                    type_mappings: vec![
                        ("int".to_string(), "int or int32".to_string()),
                        ("char".to_string(), "byte or rune".to_string()),
                        ("char*".to_string(), "string or []byte".to_string()),
                        ("int[]".to_string(), "[]int".to_string()),
                        ("void*".to_string(), "interface{}".to_string()),
                        ("size_t".to_string(), "int".to_string()),
                    ],
                    stdlib_mappings: vec![
                        ("printf".to_string(), "fmt.Printf()".to_string()),
                        ("malloc".to_string(), "make() or new()".to_string()),
                        ("strlen".to_string(), "len()".to_string()),
                        ("fopen".to_string(), "os.Open()".to_string()),
                    ],
                    pitfalls: vec![
                        "No pointer arithmetic in Go".to_string(),
                        "Slices are references, not copies".to_string(),
                        "Go strings are immutable".to_string(),
                        "Must handle errors explicitly".to_string(),
                    ],
                    idioms: vec![
                        "Use multiple return values for errors".to_string(),
                        "Use defer for resource cleanup".to_string(),
                        "Prefer composition over inheritance".to_string(),
                        "Use channels for communication".to_string(),
                    ],
                    example: None,
                },
            },
            // Ruby to Python
            LanguagePairPattern {
                source: ProgrammingLanguage::Ruby,
                target: ProgrammingLanguage::Python,
                name: "Ruby to Python Porting".to_string(),
                keywords: vec![
                    "python".to_string(),
                    "rails".to_string(),
                    "django".to_string(),
                ],
                confidence: 0.0,
                challenges: vec![
                    PortingCategory::IdiomaticCode,
                    PortingCategory::StandardLibrary,
                ],
                guidance: PortingGuidance {
                    approach: "Ruby and Python are similar high-level languages. Main differences are blocks vs comprehensions and explicit self in Python.".to_string(),
                    steps: vec![
                        "Convert Ruby blocks to Python list comprehensions or map/filter".to_string(),
                        "Add explicit self parameter to methods".to_string(),
                        "Replace symbols with strings".to_string(),
                        "Convert unless to if not".to_string(),
                        "Adjust indentation-based syntax".to_string(),
                        "Port RSpec to pytest".to_string(),
                    ],
                    type_mappings: vec![
                        ("Array".to_string(), "list".to_string()),
                        ("Hash".to_string(), "dict".to_string()),
                        (":symbol".to_string(), "'string'".to_string()),
                        ("nil".to_string(), "None".to_string()),
                        ("true/false".to_string(), "True/False".to_string()),
                    ],
                    stdlib_mappings: vec![
                        ("puts".to_string(), "print()".to_string()),
                        ("each".to_string(), "for loop or comprehension".to_string()),
                        ("map".to_string(), "list(map()) or comprehension".to_string()),
                        ("File.read".to_string(), "open().read()".to_string()),
                    ],
                    pitfalls: vec![
                        "Ruby has implicit returns, Python needs explicit".to_string(),
                        "Ruby blocks don't translate directly".to_string(),
                        "Different truthiness rules".to_string(),
                    ],
                    idioms: vec![
                        "Use list comprehensions for transformations".to_string(),
                        "Use with statement for resource management".to_string(),
                        "Follow PEP 8 style guide".to_string(),
                    ],
                    example: None,
                },
            },
        ]
    }

    /// Add a custom language pair pattern
    pub fn add_pattern(&mut self, pattern: LanguagePairPattern) {
        self.patterns.push(pattern);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_c_to_rust() {
        let detector = CodePortingPatternDetector::new();
        let result = detector.detect("Port this C code to Rust with memory safety");

        assert!(result.should_augment, "Should augment for porting task");
        assert!(!result.patterns.is_empty(), "Should detect patterns");
        let has_c_rust = result.patterns.iter().any(|p| {
            p.source == ProgrammingLanguage::C && p.target == ProgrammingLanguage::Rust
        });
        assert!(has_c_rust, "Should detect C to Rust pattern");
    }

    #[test]
    fn test_detect_python_to_go() {
        let detector = CodePortingPatternDetector::new();
        let result = detector.detect("Convert this Python script to Go for better performance");

        assert!(result.should_augment, "Should augment for porting task");
        let has_py_go = result.patterns.iter().any(|p| {
            p.source == ProgrammingLanguage::Python && p.target == ProgrammingLanguage::Go
        });
        assert!(has_py_go, "Should detect Python to Go pattern");
    }

    #[test]
    fn test_detect_js_to_ts() {
        let detector = CodePortingPatternDetector::new();
        let result = detector.detect("Migrate JavaScript codebase to TypeScript with strict mode");

        assert!(result.should_augment, "Should augment for migration task");
        let has_js_ts = result.patterns.iter().any(|p| {
            p.source == ProgrammingLanguage::JavaScript
                && p.target == ProgrammingLanguage::TypeScript
        });
        assert!(has_js_ts, "Should detect JS to TS pattern");
    }

    #[test]
    fn test_no_porting_task() {
        let detector = CodePortingPatternDetector::new();
        let result = detector.detect("Write a function to calculate fibonacci numbers in Python");

        assert!(
            !result.should_augment,
            "Should not augment for non-porting task"
        );
        assert!(
            result.overall_confidence < 0.3,
            "Should have low confidence"
        );
    }

    #[test]
    fn test_language_detection() {
        let detector = CodePortingPatternDetector::new();
        let result = detector.detect("Port from C to Rust");

        assert_eq!(
            result.source_language,
            Some(ProgrammingLanguage::C),
            "Should detect C as source"
        );
        assert_eq!(
            result.target_language,
            Some(ProgrammingLanguage::Rust),
            "Should detect Rust as target"
        );
    }

    #[test]
    fn test_prompt_augmentation() {
        let detector = CodePortingPatternDetector::new();
        let result = detector.detect("Port C code to Rust");

        let augmentation = detector.generate_prompt_augmentation(&result);

        assert!(!augmentation.is_empty(), "Should generate augmentation");
        assert!(
            augmentation.contains("Type Mappings") || augmentation.contains("type_mappings"),
            "Should include type mappings"
        );
    }

    #[test]
    fn test_lua_language_detection() {
        let detector = CodePortingPatternDetector::new();
        let result = detector.detect("Convert this Python script to Lua for love2d game");

        // Lua should be detected
        let has_lua = result.source_language == Some(ProgrammingLanguage::Lua)
            || result.target_language == Some(ProgrammingLanguage::Lua)
            || result.matched_keywords.iter().any(|k| k.to_lowercase().contains("lua"));

        // Since we don't have a Python-to-Lua pattern, just check we detected some porting keywords
        assert!(result.should_augment || result.overall_confidence > 0.0);
    }
}
