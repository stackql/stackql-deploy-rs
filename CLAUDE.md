# Claude Code

Read and follow [AGENTS.md](AGENTS.md). It is the canonical guidance for this
repository; do not duplicate project rules here.

Claude Code specifics:

- Local tool permissions live in `.claude/settings.local.json`, which is
  gitignored and per-developer. Its deny list (`~/.ssh`, `~/.aws`, `.env`,
  `.env.*`) mirrors the secrets rules in `AGENTS.md`.
- The `Claude PR Assistant` workflow (`.github/workflows/claude.yaml`)
  responds to `@claude` mentions on issues and pull requests and also reads
  `AGENTS.md` through this file.
