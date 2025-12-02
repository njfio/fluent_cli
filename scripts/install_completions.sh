#!/bin/bash
# Install shell completions for Fluent CLI
# Supports Bash, Zsh, and Fish

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Detect shell
detect_shell() {
    if [ -n "$BASH_VERSION" ]; then
        echo "bash"
    elif [ -n "$ZSH_VERSION" ]; then
        echo "zsh"
    elif [ -n "$FISH_VERSION" ]; then
        echo "fish"
    else
        # Fallback to shell from environment
        basename "$SHELL"
    fi
}

# Install bash completions
install_bash() {
    echo "Installing Bash completions..."

    # Try user-level directory first
    COMPLETION_DIR="$HOME/.local/share/bash-completion/completions"
    mkdir -p "$COMPLETION_DIR"

    fluent completions --shell bash > "$COMPLETION_DIR/fluent"

    echo -e "${GREEN}✓${NC} Bash completions installed to: $COMPLETION_DIR/fluent"
    echo "To activate in current shell, run:"
    echo "  source $COMPLETION_DIR/fluent"
    echo "Or restart your shell."
}

# Install zsh completions
install_zsh() {
    echo "Installing Zsh completions..."

    COMPLETION_DIR="$HOME/.zfunc"
    mkdir -p "$COMPLETION_DIR"

    fluent completions --shell zsh > "$COMPLETION_DIR/_fluent"

    echo -e "${GREEN}✓${NC} Zsh completions installed to: $COMPLETION_DIR/_fluent"

    # Check if fpath is configured
    if ! grep -q "fpath+=.*\.zfunc" "$HOME/.zshrc" 2>/dev/null; then
        echo -e "${YELLOW}!${NC} Add the following to your ~/.zshrc:"
        echo "  fpath+=~/.zfunc"
        echo "  autoload -Uz compinit && compinit"
        echo ""
        read -p "Add to ~/.zshrc automatically? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo "" >> "$HOME/.zshrc"
            echo "# Fluent CLI completions" >> "$HOME/.zshrc"
            echo "fpath+=~/.zfunc" >> "$HOME/.zshrc"
            echo "autoload -Uz compinit && compinit" >> "$HOME/.zshrc"
            echo -e "${GREEN}✓${NC} Updated ~/.zshrc"
        fi
    fi

    echo "To activate, restart your shell or run:"
    echo "  source ~/.zshrc"
}

# Install fish completions
install_fish() {
    echo "Installing Fish completions..."

    COMPLETION_DIR="$HOME/.config/fish/completions"
    mkdir -p "$COMPLETION_DIR"

    fluent completions --shell fish > "$COMPLETION_DIR/fluent.fish"

    echo -e "${GREEN}✓${NC} Fish completions installed to: $COMPLETION_DIR/fluent.fish"
    echo "Fish will automatically load completions. Start a new shell or run:"
    echo "  source ~/.config/fish/config.fish"
}

# Main
main() {
    echo "Fluent CLI - Shell Completions Installer"
    echo "========================================"
    echo ""

    # Check if fluent is available
    if ! command -v fluent &> /dev/null; then
        echo -e "${RED}✗${NC} 'fluent' command not found."
        echo "Please install Fluent CLI first or add it to your PATH."
        exit 1
    fi

    # Detect shell
    DETECTED_SHELL=$(detect_shell)

    # Allow user to override
    if [ $# -eq 0 ]; then
        echo "Detected shell: $DETECTED_SHELL"
        read -p "Install completions for this shell? [Y/n] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Nn]$ ]]; then
            echo "Available shells: bash, zsh, fish, all"
            read -p "Enter shell name: " SELECTED_SHELL
        else
            SELECTED_SHELL="$DETECTED_SHELL"
        fi
    else
        SELECTED_SHELL="$1"
    fi

    # Install for selected shell
    case "$SELECTED_SHELL" in
        bash)
            install_bash
            ;;
        zsh)
            install_zsh
            ;;
        fish)
            install_fish
            ;;
        all)
            echo "Installing completions for all supported shells..."
            echo ""
            install_bash
            echo ""
            install_zsh
            echo ""
            install_fish
            ;;
        *)
            echo -e "${RED}✗${NC} Unsupported shell: $SELECTED_SHELL"
            echo "Supported shells: bash, zsh, fish, all"
            exit 1
            ;;
    esac

    echo ""
    echo -e "${GREEN}✓${NC} Installation complete!"
}

# Run main
main "$@"
