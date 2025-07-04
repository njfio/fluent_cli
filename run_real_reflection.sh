#!/bin/bash

echo "🧠 Fluent CLI Real LLM Reflection Demo Setup"
echo "============================================="
echo

# Check if any API keys are already set
if [[ -n "$OPENAI_API_KEY" ]]; then
    echo "✅ OpenAI API key found"
    PROVIDER="OpenAI"
elif [[ -n "$ANTHROPIC_API_KEY" ]]; then
    echo "✅ Anthropic API key found"
    PROVIDER="Anthropic"
elif [[ -n "$GOOGLE_API_KEY" ]]; then
    echo "✅ Google API key found"
    PROVIDER="Google"
else
    echo "❌ No API keys found!"
    echo
    echo "Please set one of the following API keys:"
    echo
    echo "For OpenAI (recommended):"
    echo "  export OPENAI_API_KEY=\"sk-your-openai-key-here\""
    echo
    echo "For Anthropic:"
    echo "  export ANTHROPIC_API_KEY=\"your-anthropic-key-here\""
    echo
    echo "For Google:"
    echo "  export GOOGLE_API_KEY=\"your-google-key-here\""
    echo
    echo "Then run this script again: ./run_real_reflection.sh"
    echo
    echo "🔑 To get API keys:"
    echo "   OpenAI: https://platform.openai.com/api-keys"
    echo "   Anthropic: https://console.anthropic.com/"
    echo "   Google: https://makersuite.google.com/app/apikey"
    exit 1
fi

echo "🚀 Using $PROVIDER for LLM-powered reflection"
echo

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found! Please install Rust: https://rustup.rs/"
    exit 1
fi

echo "🔧 Building and running real LLM reflection demo..."
echo

# Run the example
cargo run --example real_reflection_with_llm

echo
echo "🎉 Demo complete!"
echo
echo "💡 What you just saw:"
echo "   - Real LLM integration with $PROVIDER"
echo "   - Intelligent self-reflection triggering"
echo "   - LLM-generated insights and strategies"
echo "   - Performance analysis with actual AI"
echo
echo "🔧 To run other examples:"
echo "   cargo run --example reflection_demo          # Mock reflection demo"
echo "   cargo run --example state_management_demo    # State persistence demo"
echo
echo "🚀 To use the CLI with reflection:"
echo "   cargo run -- openai --agentic --goal \"Analyze this codebase\" --enable-tools --max-iterations 10"
