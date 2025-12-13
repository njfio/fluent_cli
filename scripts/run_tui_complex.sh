#!/usr/bin/env bash
set -euo pipefail
export FLUENT_RUN_ID="llm-inference-$(date +%s)-$$"
export FLUENT_STATE_STORE="./state"
export FLUENT_TUI_MAX_LOGS="400"
mkdir -p "$FLUENT_STATE_STORE"
mkdir -p ./outputs/research_llm_inference
cargo run -p fluent-cli -- agent --agentic --goal-file examples/goals/complex_research_goal.toml --enable-tools --reflection --max-iterations 30 --tui
