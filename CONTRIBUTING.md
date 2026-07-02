# Contributing to devnote

Thank you for considering contributing to devnote! We welcome contributions from everyone.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/devnote.git`
3. Create a branch: `git checkout -b your-feature-branch`
4. Install dependencies: `bun install`
5. Start development: `bun run tauri dev`

## Development Workflow

- Run type checks: `bun run check`
- Run Rust lint: `cd src-tauri && cargo clippy -- -D warnings`
- Run Rust tests: `cd src-tauri && cargo test`

## Code Style

- Follow the conventions in `AGENTS.md` and `DESIGN.md`
- Use CSS custom properties for all styling — no inline hex values
- TypeScript: no `any`, use proper types
- Svelte 5 runes syntax ($state, $derived, $effect) — not legacy `$:` syntax
- Rust: no `unwrap()` in production paths, use `?` + `map_err`

## Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add new feature
fix: fix a bug
docs: update documentation
refactor: refactor code
chore: maintenance tasks
test: add or update tests
style: formatting, linting
```

## Pull Request Process

1. Ensure your code passes type checks and lint
2. Update documentation if needed
3. Submit a PR with a clear description of changes
4. A maintainer will review and provide feedback

## Code of Conduct

Please be respectful and constructive. Harassment or toxic behavior will not be tolerated.
