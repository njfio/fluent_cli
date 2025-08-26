use super::ToolExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use fluent_core::traits::Engine;
use fluent_core::types::Request;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

/// High-level workflow tools that orchestrate multi-step operations
pub struct WorkflowExecutor {
    engine: Arc<Box<dyn Engine>>,
}

impl WorkflowExecutor {
    pub fn new(engine: Arc<Box<dyn Engine>>) -> Self { Self { engine } }

    async fn llm(&self, prompt: String) -> Result<String> {
        let req = Request { flowname: "workflow".to_string(), payload: prompt };
        let resp = Pin::from(self.engine.execute(&req)).await?;
        Ok(resp.content)
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<String> {
        let p = std::path::Path::new(path);
        if let Some(parent) = p.parent() { tokio::fs::create_dir_all(parent).await?; }
        tokio::fs::write(p, content).await?;
        Ok(format!("Successfully wrote to {}", path))
    }

    async fn concat_files(&self, paths: Vec<String>, dest: &str, sep: &str) -> Result<String> {
        let mut combined = String::new();
        for (i, p) in paths.iter().enumerate() {
            let s = tokio::fs::read_to_string(p).await.unwrap_or_default();
            if i > 0 { combined.push_str(sep); }
            combined.push_str(&s);
        }
        self.write_file(dest, &combined).await
    }

    async fn generate_book_outline(&self, goal: &str, out_path: &str) -> Result<String> {
        let prompt = format!("Produce a detailed book outline for the following goal. Use Markdown with chapters and sections.\n\nGoal:\n{}", goal);
        let content = self.llm(prompt).await?;
        self.write_file(out_path, &content).await
    }

    async fn generate_toc(&self, outline_path: &str, chapters: usize, out_path: &str) -> Result<String> {
        let prompt = format!(
            "Generate a Markdown Table of Contents (TOC) based on an outline at {} and {} chapters (ch_01..ch_{:02}). Include anchors.",
            outline_path, chapters, chapters
        );
        let content = self.llm(prompt).await?;
        self.write_file(out_path, &content).await
    }

    async fn assemble_book(&self, base: &str, chapters: usize) -> Result<String> {
        let mut paths = vec![format!("{}/toc.md", base)];
        for i in 1..=chapters { paths.push(format!("{}/ch_{:02}.md", base, i)); }
        self.concat_files(paths, &format!("{}/book.md", base), "\n\n---\n\n").await
    }

    async fn research_generate(&self, kind: &str, goal: &str, out_path: &str) -> Result<String> {
        let (instruction, details) = match kind {
            "outline" => ("Create a research outline", "sections, key questions, sources"),
            "notes" => ("Write research notes", "numbered citations [1], [2], quotes"),
            "summary" => ("Write an executive summary", "key findings, references list"),
            _ => return Err(anyhow!("Unknown research kind: {}", kind)),
        };
        let prompt = format!(
            "{} in Markdown for the following topic. Include {}.\n\nTopic:\n{}",
            instruction, details, goal
        );
        let content = self.llm(prompt).await?;
        self.write_file(out_path, &content).await
    }
}

#[async_trait]
impl ToolExecutor for WorkflowExecutor {
    async fn execute_tool(&self, tool_name: &str, parameters: &HashMap<String, serde_json::Value>) -> Result<String> {
        match tool_name {
            // Long-form tools
            "generate_book_outline" => {
                let goal = parameters.get("goal").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("goal required"))?;
                let out_path = parameters.get("out_path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("out_path required"))?;
                self.generate_book_outline(goal, out_path).await
            }
            "generate_toc" => {
                let outline_path = parameters.get("outline_path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("outline_path required"))?;
                let chapters = parameters.get("chapters").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                let out_path = parameters.get("out_path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("out_path required"))?;
                self.generate_toc(outline_path, chapters, out_path).await
            }
            "assemble_book" => {
                let base = parameters.get("base").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("base required"))?;
                let chapters = parameters.get("chapters").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                self.assemble_book(base, chapters).await
            }

            // Research tools
            "research_generate_outline" => {
                let goal = parameters.get("goal").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("goal required"))?;
                let out_path = parameters.get("out_path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("out_path required"))?;
                self.research_generate("outline", goal, out_path).await
            }
            "research_generate_notes" => {
                let goal = parameters.get("goal").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("goal required"))?;
                let out_path = parameters.get("out_path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("out_path required"))?;
                self.research_generate("notes", goal, out_path).await
            }
            "research_generate_summary" => {
                let goal = parameters.get("goal").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("goal required"))?;
                let out_path = parameters.get("out_path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("out_path required"))?;
                self.research_generate("summary", goal, out_path).await
            }
            _ => Err(anyhow!("Unknown workflow tool: {}", tool_name)),
        }
    }

    fn get_available_tools(&self) -> Vec<String> {
        vec![
            "generate_book_outline".to_string(),
            "generate_toc".to_string(),
            "assemble_book".to_string(),
            "research_generate_outline".to_string(),
            "research_generate_notes".to_string(),
            "research_generate_summary".to_string(),
        ]
    }

    fn get_tool_description(&self, tool_name: &str) -> Option<String> {
        let d = match tool_name {
            "generate_book_outline" => "Generate a book outline using LLM and write to file",
            "generate_toc" => "Generate a Markdown TOC and write to file",
            "assemble_book" => "Concatenate TOC and chapter files into book.md",
            "research_generate_outline" => "Generate research outline and write to file",
            "research_generate_notes" => "Generate research notes with citations and write to file",
            "research_generate_summary" => "Generate research executive summary and write to file",
            _ => return None,
        };
        Some(d.to_string())
    }

    fn validate_tool_request(&self, _tool_name: &str, _parameters: &HashMap<String, serde_json::Value>) -> Result<()> {
        Ok(())
    }
}
