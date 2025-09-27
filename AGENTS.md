# Repository Guidelines

## Project Structure & Module Organization
- Core source lives in `docs/deepagents/src/deepagents/`, grouped by concern (`graph.py`, `middleware.py`, `tools.py`, etc.).
- Public packaging metadata and docs sit under `docs/deepagents/pyproject.toml` and `docs/deepagents/README.md`.
- Tests are in `docs/deepagents/tests/` with shared fixtures and utilities in `tests/utils.py`.
- Reference example agents and assets (e.g., research workflow, diagrams) reside in `docs/deepagents/examples/` and `docs/deepagents/deep_agents.png`.

## Build, Test, and Development Commands
- `cd docs/deepagents && pip install -e .[dev]` installs runtime and developer dependencies.
- `pytest` from the same directory runs the unit test suite across middleware, HITL, and agent bindings.
- `python -m examples.research.research_agent` executes the sample research agent; set `TAVILY_API_KEY` beforehand.

## Coding Style & Naming Conventions
- Follow idiomatic Python 3.11+ with 4-space indentation and type hints for public interfaces.
- Prefer descriptive module-level names (`create_deep_agent`, `PlanningMiddleware`) that mirror existing patterns.
- Keep prompts and tool descriptions in `prompts.py`, using UPPER_SNAKE_CASE constants; attach brief comments only for non-obvious logic.

## Testing Guidelines
- Use `pytest` for new tests; mirror current naming with `test_*.py` files and `Test*` classes.
- Co-locate fixtures in `tests/utils.py` or introduce module-local fixtures when tightly scoped.
- Ensure new features exercise agent orchestration and state updates; aim for deterministic assertions (avoid live API calls).

## Commit & Pull Request Guidelines
- Write commits in imperative mood (`Add HITL coverage`, `Refine subagent middleware model handling`).
- Each PR should: describe the change, reference related issues or tickets, note testing commands, and include screenshots or logs when behavior changes.
- Keep unrelated refactors separate from feature or bug-fix patches to simplify review.

## Security & Configuration Tips
- Store API keys (e.g., Anthropic, Tavily) in AWS Secrets Manager or environment variables; never commit secrets.
- When adding customer-specific tools, isolate configuration under environment-driven settings so shared agents remain generic.
