# CI/CD: Automated Completions Regeneration

This document describes how to add automated shell completions regeneration to CI/CD workflows.

## Overview

Shell completions should be regenerated whenever the CLI structure changes (new commands, flags, or subcommands). This ensures users always have up-to-date completions.

## GitHub Actions Integration

### Option 1: Generate on Release (Recommended)

Add a step to your release workflow to include completions in release artifacts:

```yaml
# .github/workflows/rust.yml (in deploy job)
deploy:
  if: startsWith(github.ref, 'refs/tags/')
  needs: build
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable

    - name: Build release binary
      run: cargo build --release

    - name: Generate shell completions
      run: |
        mkdir -p completions
        ./target/release/fluent completions --shell bash > completions/fluent.bash
        ./target/release/fluent completions --shell zsh > completions/_fluent
        ./target/release/fluent completions --shell fish > completions/fluent.fish
        ./target/release/fluent completions --shell powershell > completions/fluent.ps1

    - name: Package completions
      run: |
        tar -czf completions.tar.gz completions/
        zip -r completions.zip completions/

    - name: Download artifacts
      uses: actions/download-artifact@v4
      with:
        name: result
        path: ./artifacts

    - name: Release
      uses: softprops/action-gh-release@v1
      with:
        files: |
          ./artifacts/*.tar.gz
          completions.tar.gz
          completions.zip
```

### Option 2: Validate in CI

Add a job to validate completions are generated without errors:

```yaml
# .github/workflows/rust.yml
completions:
  name: Validate Completions
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable

    - name: Build CLI
      run: cargo build --release

    - name: Test completion generation
      run: |
        echo "Testing Bash completions..."
        ./target/release/fluent completions --shell bash > /dev/null || exit 1

        echo "Testing Zsh completions..."
        ./target/release/fluent completions --shell zsh > /dev/null || exit 1

        echo "Testing Fish completions..."
        ./target/release/fluent completions --shell fish > /dev/null || exit 1

        echo "Testing PowerShell completions..."
        ./target/release/fluent completions --shell powershell > /dev/null || exit 1

        echo "✓ All completions generated successfully"

    - name: Upload completions as artifact
      uses: actions/upload-artifact@v4
      with:
        name: shell-completions
        path: completions/
```

### Option 3: Auto-commit Updated Completions

Automatically commit updated completions to the repository:

```yaml
# .github/workflows/update-completions.yml
name: Update Shell Completions

on:
  push:
    branches: [main]
    paths:
      - 'crates/fluent-cli/src/cli.rs'
      - 'crates/fluent-cli/src/cli_builder.rs'
      - 'crates/fluent-cli/src/commands/**'

jobs:
  update-completions:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build CLI
        run: cargo build --release

      - name: Generate completions
        run: |
          mkdir -p completions
          ./target/release/fluent completions --shell bash > completions/fluent.bash
          ./target/release/fluent completions --shell zsh > completions/_fluent
          ./target/release/fluent completions --shell fish > completions/fluent.fish
          ./target/release/fluent completions --shell powershell > completions/fluent.ps1

      - name: Check for changes
        id: changes
        run: |
          git diff --quiet completions/ || echo "changed=true" >> $GITHUB_OUTPUT

      - name: Commit updated completions
        if: steps.changes.outputs.changed == 'true'
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add completions/
          git commit -m "chore: regenerate shell completions"
          git push
```

## Current Implementation

The current `.github/workflows/rust.yml` workflow includes completions in release artifacts via line 102:

```yaml
tar -czf ./artifacts/$NAME-$TARGET-$TAG.tar.gz $EXEC default_config_test.json amber.yaml ... fluent_autocomplete.sh fluent_autocomplete.ps1
```

**However**, these are legacy autocomplete scripts, not the modern `fluent completions` output.

### Recommended Update

Replace the legacy scripts with generated completions:

```yaml
# In the build job, after "Run build" step:
- name: Generate shell completions
  run: |
    mkdir -p completions
    ./target/$TARGET/release/$EXEC completions --shell bash > completions/fluent.bash
    ./target/$TARGET/release/$EXEC completions --shell zsh > completions/_fluent
    ./target/$TARGET/release/$EXEC completions --shell fish > completions/fluent.fish
    ./target/$TARGET/release/$EXEC completions --shell powershell > completions/fluent.ps1

# Update the "Compress" step:
- name: Compress
  run: |
    mkdir -p ./artifacts
    if [[ $OS =~ ^windows.*$ ]]; then
        EXEC=$NAME.exe
    else
        EXEC=$NAME
    fi
    if [[ $GITHUB_REF_TYPE =~ ^tag$ ]]; then
      TAG=$GITHUB_REF_NAME
    else
      TAG=$GITHUB_SHA
    fi
    mv ./target/$TARGET/release/$EXEC $EXEC
    tar -czf ./artifacts/$NAME-$TARGET-$TAG.tar.gz $EXEC completions/
```

## GitLab CI Integration

For GitLab CI, add similar steps to `.gitlab-ci.yml`:

```yaml
generate-completions:
  stage: build
  script:
    - cargo build --release
    - mkdir -p completions
    - ./target/release/fluent completions --shell bash > completions/fluent.bash
    - ./target/release/fluent completions --shell zsh > completions/_fluent
    - ./target/release/fluent completions --shell fish > completions/fluent.fish
    - ./target/release/fluent completions --shell powershell > completions/fluent.ps1
  artifacts:
    paths:
      - completions/
    expire_in: 30 days
```

## Local Development

Developers should regenerate completions after CLI changes:

```bash
# After modifying CLI commands
cargo build --release

# Regenerate completions
mkdir -p completions
./target/release/fluent completions --shell bash > completions/fluent.bash
./target/release/fluent completions --shell zsh > completions/_fluent
./target/release/fluent completions --shell fish > completions/fluent.fish
./target/release/fluent completions --shell powershell > completions/fluent.ps1

# Commit if changed
git add completions/
git commit -m "chore: regenerate shell completions"
```

### Pre-commit Hook (Optional)

Add a pre-commit hook to regenerate completions automatically:

```bash
# .git/hooks/pre-commit
#!/bin/bash

# Check if CLI files changed
if git diff --cached --name-only | grep -q "crates/fluent-cli/src/cli"; then
    echo "CLI changed, regenerating completions..."

    cargo build --release 2>/dev/null || {
        echo "Build failed, skipping completion regeneration"
        exit 0
    }

    mkdir -p completions
    ./target/release/fluent completions --shell bash > completions/fluent.bash
    ./target/release/fluent completions --shell zsh > completions/_fluent
    ./target/release/fluent completions --shell fish > completions/fluent.fish
    ./target/release/fluent completions --shell powershell > completions/fluent.ps1

    git add completions/
    echo "✓ Completions regenerated"
fi
```

## Testing Completions in CI

Add tests to verify completions are valid:

```yaml
- name: Test completions syntax
  run: |
    # Test Bash completion syntax
    bash -n completions/fluent.bash || exit 1

    # Test Zsh completion (basic check)
    test -f completions/_fluent || exit 1

    # Test Fish completion syntax
    fish -n completions/fluent.fish 2>/dev/null || exit 1

    # Test PowerShell completion syntax (if PowerShell available)
    if command -v pwsh &> /dev/null; then
        pwsh -Command "Test-Path completions/fluent.ps1" || exit 1
    fi

    echo "✓ All completion files are valid"
```

## Benefits of CI Integration

1. **Always Up-to-Date**: Completions are regenerated with every release
2. **Quality Assurance**: Validates completions are generated without errors
3. **Distribution**: Includes completions in release artifacts
4. **Documentation**: Provides completions for package managers
5. **Developer Experience**: Automates manual process

## Migration from Legacy Scripts

The repository currently includes legacy scripts:
- `fluent_autocomplete.sh` (Bash)
- `fluent_autocomplete.ps1` (PowerShell)

These should be:
1. Removed from repository (after deprecation period)
2. Replaced with generated completions in CI artifacts
3. Documented as deprecated in README

Users should migrate to `fluent completions` command instead.

## Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [clap_complete Documentation](https://docs.rs/clap_complete/)
- [Semantic Versioning](https://semver.org/) - for tagging releases
