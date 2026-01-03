# Changelog

All notable changes to this project are documented in this file.

## [0.0.29] - 2026-01-05

### Added
- **TOON Format Support**: Token-Oriented Object Notation for 30-60% token reduction
  - `PromptFormat` enum (`Json`, `Toon`)
  - `ConfigurableAgentBuilder::with_prompt_format()`
  - `ToonEncoder` utility for direct encoding
  - Feature-gated via `toon` feature flag
- **System Prompt Override**: `with_system_prompt()` for complete prompt control
- **TOON Documentation**: Comprehensive guide at `docs/TOON_FORMAT.md`
- **TOON Example**: `examples/toon-format-demo`

### Changed
- `DeepAgentConfig` and `DeepAgentPromptMiddleware` now support `PromptFormat`

## [0.0.28] - 2026-01-03

### Added
- **Configurable System Prompt**: `with_system_prompt()` method
- Improved documentation and README

### Fixed
- System prompt handling in middleware

## [0.0.27] - 2025-12-28

### Added
- Max iterations configuration
- Improved error messages

## [0.0.26] - 2025-12-20

### Added
- Event system improvements
- Better streaming support

## [0.0.25] - 2025-12-15

### Added
- Sub-agent support
- Summarization middleware

## Earlier Releases

See [GitHub Releases](https://github.com/yafatek/rust-deep-agents-sdk/releases) for complete history.

