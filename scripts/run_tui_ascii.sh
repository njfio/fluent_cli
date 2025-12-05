#!/usr/bin/env bash
set -euo pipefail
export FLUENT_RUN_ID="ascii-$(date +%s)-$$"
export FLUENT_STATE_STORE="./state"
export FLUENT_TUI_MAX_LOGS="400"
export FLUENT_USE_OLD_TUI=1
export NO_COLOR=1
mkdir -p "$FLUENT_STATE_STORE"
mkdir -p ./outputs/research_llm_inference
cargo run -p fluent-cli -- agent --agentic --goal-file examples/goals/complex_research_goal.toml --enable-tools --reflection --max-iterations 30 --tui
