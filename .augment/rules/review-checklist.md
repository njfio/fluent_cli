---
type: "always_apply"
---

Code Review Checklist for Fluent CLI
General
 Does the code follow coding standards (formatting, naming, error handling)?
 Are there any bad habits (unwraps, ignored errors, duplicated code)?
 Is the code modular and well-organized?
Functionality
 Does the change work as intended? Test locally if possible.
 Are edge cases handled (e.g., invalid inputs, network failures)?
 Does it integrate correctly with existing features (e.g., new engine compatible with CLI)?
Testing
 Are there new tests for the change?
 Do all tests pass? Check coverage.
 Are integration tests updated if CLI behavior changes?
Documentation
 Are doc comments added/updated for new/changed code?
 Is user-facing documentation (guides, examples) updated?
 Is the CHANGELOG updated if applicable?
Security
 Are inputs validated and sanitized?
 No hard-coded secrets or sensitive data?
 Dependencies updated and secure?
 Any potential vulnerabilities (e.g., command injection in shell executions)?
Performance
 No unnecessary allocations or inefficiencies?
 Does it scale for large inputs (e.g., big pipelines)?
Style and Readability
 Code is clean, commented where needed.
 No magic numbers; use constants.
 Functions are short and focused.
Project Fit
 Aligns with project goals (automation, productivity, security)?
 Follows project structure and guidelines?
Reviewers: Provide constructive feedback. Authors: Address all items before merge.