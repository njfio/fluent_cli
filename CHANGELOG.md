# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **String Replace Editor Tool**: Advanced file editing capabilities with surgical precision
  - Multiple occurrence modes (First, Last, All, Indexed)
  - Line range targeting for precise edits
  - Dry run previews for safe operation planning
  - Automatic timestamped backup creation
  - Comprehensive security validation and path restrictions
  - Case sensitivity control
  - Integration with agent tool registry
- **Enhanced Tool System**: Production-ready tool registry with comprehensive file operations
- **Tool Registry Integration**: Automatic registration of all standard tools
- **Comprehensive Test Suite**: Unit tests, integration tests, and validation tests for string replace functionality

### Changed
- **Tool System**: Upgraded from experimental to production-ready status
- **Agent Configuration**: Enhanced tool configuration with security constraints
- **Documentation**: Updated README with comprehensive tool system documentation

### Fixed
- **Example Compilation**: Removed problematic demo examples that caused test failures
- **API Consistency**: Updated working examples to use current API methods
- **Configuration Structure**: Fixed tool configuration to match current schema

### Security
- **Path Validation**: Comprehensive path restriction and validation system
- **Input Sanitization**: All tool parameters validated before execution
- **Backup Protection**: Automatic backup creation for file safety
- **Resource Limits**: Configurable file size and operation limits

## [0.1.0] - 2024-01-XX

### Added
- Initial release of Fluent CLI
- Multi-provider LLM support (OpenAI, Anthropic, Google, etc.)
- Basic agent system with memory
- MCP (Model Context Protocol) integration
- Pipeline execution system
- Configuration management
- Caching system
- Basic tool system (experimental)

### Features
- **LLM Providers**: OpenAI, Anthropic, Google Gemini, Cohere, Mistral, Perplexity, Groq
- **Agent System**: Interactive agent sessions with SQLite memory
- **MCP Integration**: Basic client and server capabilities
- **Pipeline System**: YAML-based multi-step workflows
- **Configuration**: YAML-based engine and pipeline configuration
- **Caching**: Optional request caching for improved performance

### Experimental
- Basic agent loop implementation
- Limited tool system functionality
- Prototype MCP integration
- Basic memory system with SQLite storage
