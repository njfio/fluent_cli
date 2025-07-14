---
type: "agent_requested"
description: "Follow this exact protocol step-by-step to ensure the codebase is in excellent shape, all documentation is relevant and up-to-date, and a changelog is maintained before creating a new pull request (PR). Do not skip any steps. Report back on each step's outcome for verification."
---
repare and Create a High-Quality Pull Request

Follow this exact protocol step-by-step to ensure the codebase is in excellent shape, all documentation is relevant and up-to-date, and a changelog is maintained before creating a new pull request (PR). Do not skip any steps. Report back on each step's outcome for verification.

Review and Optimize the Codebase:
Perform a full code review: Check for code quality, bugs, inefficiencies, and adherence to best practices (e.g., using linters like ESLint for JS or pylint for Python if applicable).
Run all tests (unit, integration, etc.) to ensure 100% pass rate. Fix any failures.
Ensure the code is modular, readable, and follows the project's style guide (e.g., PEP 8 for Python).
Remove any dead code, unused variables, or deprecated features.
Confirm the branch is up-to-date with the main branch (e.g., via git pull origin main and resolve conflicts).
Update and Prune Documentation:
Review all documentation files (e.g., README.md, API docs, user guides, inline comments).
Update any sections that are outdated features, APIs, or instructions to match the current codebase.
Remove any documentation files or sections that are no longer relevant (e.g., docs for removed features). If a file is partially irrelevant, refactor it instead of deleting.
Add new documentation where needed (e.g., for new features or changes).
Ensure docs are clear, concise, and formatted consistently (e.g., use Markdown best practices).
If the project lacks one, create or update a CHANGELOG.md file following the Keep a Changelog format. Append entries for this change under sections like "Added," "Changed," "Fixed," or "Removed," including version numbers and dates.
Stage and Commit Changes:
Stage all modified, added, or deleted files (e.g., via git add .).
Create a clean commit history: Use descriptive commit messages (e.g., "feat: Add user authentication" following Conventional Commits). Squash unnecessary commits if the history is messy.
Commit all changes with a final message summarizing the updates (e.g., "Update codebase, docs, and changelog for feature X").
Create the Pull Request:
Push the branch to the remote repository (e.g., git push origin <branch-name>).
Create a new PR on GitHub targeting the main branch.
Use a clear, concise title (e.g., "Enhance user auth with improved security").
In the PR description, include:
A summary of changes.
Links to related issues.
Before/after details for major updates.
Confirmation that tests pass, docs are updated, and changelog is appended.
Screenshots or examples if applicable.
Keep the PR small and focused—if changes are large, suggest splitting into multiple PRs.
Assign reviewers and add labels (e.g., "enhancement," "documentation").
Final Verification and Reporting:
Double-check that the codebase is stable (e.g., no lint errors, all tests pass).
If any issues arise during these steps, fix them and note them in the PR description.
Output a success message with the PR URL once created.