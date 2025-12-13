# Shell Completions Guide

This guide explains how to use and maintain shell completions for Fluent CLI.

## Overview

Fluent CLI provides native shell completion support for:
- **Bash** - Linux, macOS, and Git Bash on Windows
- **Zsh** - macOS default shell and Linux
- **Fish** - Modern shell with automatic completion loading
- **PowerShell** - Windows PowerShell and PowerShell Core

Completions are generated using `clap_complete`, which automatically creates completion scripts from the CLI definition. This ensures completions stay in sync with command changes.

## Quick Start

Generate completions for your shell:

```bash
# Bash
fluent completions --shell bash > ~/.local/share/bash-completion/completions/fluent

# Zsh
mkdir -p ~/.zfunc
fluent completions --shell zsh > ~/.zfunc/_fluent
echo 'fpath+=~/.zfunc' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc

# Fish
fluent completions --shell fish > ~/.config/fish/completions/fluent.fish

# PowerShell
fluent completions --shell powershell >> $PROFILE
```

## Detailed Installation

### Bash

**User-level installation** (recommended):
```bash
mkdir -p ~/.local/share/bash-completion/completions
fluent completions --shell bash > ~/.local/share/bash-completion/completions/fluent
```

**System-wide installation** (requires sudo):
```bash
sudo fluent completions --shell bash > /etc/bash_completion.d/fluent
```

**Reload shell:**
```bash
source ~/.bashrc
# or
exec bash
```

### Zsh

**Setup completions directory:**
```bash
mkdir -p ~/.zfunc
fluent completions --shell zsh > ~/.zfunc/_fluent
```

**Configure Zsh** (add to `~/.zshrc` if not present):
```bash
fpath+=~/.zfunc
autoload -Uz compinit && compinit
```

**Reload shell:**
```bash
source ~/.zshrc
# or
exec zsh
```

**Note**: If you see permission errors, rebuild the completion cache:
```bash
rm -f ~/.zcompdump
compinit
```

### Fish

**Install completions:**
```bash
mkdir -p ~/.config/fish/completions
fluent completions --shell fish > ~/.config/fish/completions/fluent.fish
```

Fish automatically loads completions from this directory. Start a new shell or reload:
```bash
source ~/.config/fish/config.fish
# or
exec fish
```

### PowerShell

**Option 1: Append to profile** (simplest):
```powershell
fluent completions --shell powershell >> $PROFILE
```

**Option 2: Separate file** (cleaner):
```powershell
# Generate to file
fluent completions --shell powershell > $HOME\Documents\fluent-completions.ps1

# Add to profile
Add-Content $PROFILE ". $HOME\Documents\fluent-completions.ps1"
```

**Reload profile:**
```powershell
. $PROFILE
```

**Note**: If you get an execution policy error, you may need to allow script execution:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

## Testing Completions

After installation, verify completions work:

```bash
fluent <TAB>              # Shows: agent, pipeline, tools, engine, mcp, etc.
fluent tools <TAB>        # Shows: list, describe, exec
fluent tools list <TAB>   # Shows: --category, --json, --help
fluent engine <TAB>       # Shows: list, test
fluent completions <TAB>  # Shows: --shell, --output, --help
```

## CI/CD Integration

### Generating Completions in CI

Add to your CI workflow to regenerate completions on release:

```yaml
# .github/workflows/release.yml
- name: Generate shell completions
  run: |
    mkdir -p completions
    cargo run --release -- completions --shell bash > completions/fluent.bash
    cargo run --release -- completions --shell zsh > completions/_fluent
    cargo run --release -- completions --shell fish > completions/fluent.fish
    cargo run --release -- completions --shell powershell > completions/fluent.ps1

- name: Include completions in release
  run: |
    tar -czf completions.tar.gz completions/
```

### Automated Validation

Test completions are generated without errors:

```yaml
- name: Validate completions
  run: |
    cargo build --release
    for shell in bash zsh fish powershell; do
      echo "Testing $shell completions..."
      ./target/release/fluent completions --shell $shell > /dev/null || exit 1
    done
```

## Maintenance

### When to Regenerate

Regenerate completions after:
- Adding new commands or subcommands
- Changing command flags or options
- Modifying CLI structure

### Automatic Regeneration

The `clap_complete` library automatically generates completions from the CLI definition, so they stay in sync with code changes. Simply run:

```bash
fluent completions --shell <shell>
```

### Legacy Scripts

The repository includes legacy autocomplete scripts (`fluent_autocomplete.sh` and `fluent_autocomplete.ps1`) from an older CLI version. These are **deprecated** and should not be used. Use `fluent completions` instead.

**Why?**
- Legacy scripts are manually maintained and may be outdated
- New command automatically generates accurate completions
- Supports all current subcommands (agent, tools, pipeline, etc.)
- Better error handling and edge case coverage

## Troubleshooting

### Completions Not Working

**Bash:**
- Ensure bash-completion is installed: `apt-get install bash-completion` (Linux) or `brew install bash-completion` (macOS)
- Check file location: `~/.local/share/bash-completion/completions/fluent` should exist
- Source manually: `source ~/.local/share/bash-completion/completions/fluent`

**Zsh:**
- Check `fpath`: `echo $fpath` should include `~/.zfunc`
- Rebuild completion cache: `rm -f ~/.zcompdump && compinit`
- Verify file: `cat ~/.zfunc/_fluent` should show completion script

**Fish:**
- Check file location: `~/.config/fish/completions/fluent.fish` should exist
- Fish autoloads completions; restart shell if needed
- Debug: `complete -C fluent` shows registered completions

**PowerShell:**
- Check profile: `Test-Path $PROFILE` should be True
- View profile: `cat $PROFILE` should reference completions
- Manual test: `. path\to\fluent-completions.ps1`

### Completion Shows Old Commands

Your shell may have cached old completions. Clear the cache:

```bash
# Bash
hash -r

# Zsh
rm -f ~/.zcompdump
compinit

# Fish
complete -e fluent

# PowerShell
# Restart PowerShell
```

### Permission Denied

If you see permission errors:

```bash
# Make sure the file is readable
chmod 644 ~/.local/share/bash-completion/completions/fluent

# Or regenerate
fluent completions --shell bash > ~/.local/share/bash-completion/completions/fluent
```

## Advanced Usage

### Multiple Shell Support

Install completions for all your shells:

```bash
#!/bin/bash
# install-completions.sh

# Bash
mkdir -p ~/.local/share/bash-completion/completions
fluent completions --shell bash > ~/.local/share/bash-completion/completions/fluent

# Zsh
mkdir -p ~/.zfunc
fluent completions --shell zsh > ~/.zfunc/_fluent

# Fish
mkdir -p ~/.config/fish/completions
fluent completions --shell fish > ~/.config/fish/completions/fluent.fish

echo "Completions installed! Restart your shell or source your config."
```

### Custom Installation Paths

You can install completions to custom locations:

```bash
# Custom bash location
fluent completions --shell bash --output /path/to/custom/fluent.bash
echo 'source /path/to/custom/fluent.bash' >> ~/.bashrc

# Custom zsh location
fluent completions --shell zsh --output /custom/zsh/_fluent
echo 'fpath+=(/custom/zsh)' >> ~/.zshrc
```

## Resources

- [Bash Completion Documentation](https://github.com/scop/bash-completion)
- [Zsh Completion System](https://zsh.sourceforge.io/Doc/Release/Completion-System.html)
- [Fish Completion Tutorial](https://fishshell.com/docs/current/completions.html)
- [PowerShell Tab Completion](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/register-argumentcompleter)
- [clap_complete Documentation](https://docs.rs/clap_complete/)
