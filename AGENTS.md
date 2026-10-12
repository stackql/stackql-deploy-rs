# Agent guidance

This file is the single source of truth for AI coding agents working in this
repository. Claude Code reads it through `CLAUDE.md`, GitHub Copilot through
`.github/copilot-instructions.md`, and Codex reads it directly. Keep project
rules here; the tool-specific files only point to this one.

## What this project is

`stackql-deploy` is a multi-cloud Infrastructure-as-Code CLI, written in Rust,
built on [StackQL](https://github.com/stackql/stackql). A stack is a directory
with a `stackql_manifest.yml` and `resources/*.iql` query files; the CLI
renders them with Tera templates and runs `build`, `test`, and `teardown`
against a stackql server over the PostgreSQL wire protocol. See the
[README](README.md) for the user-facing overview and
[stackql-deploy.io/docs](https://stackql-deploy.io/docs) for the full reference.

This is the v2.x Rust rewrite; the archived Python v1.x lives in
[stackql/stackql-deploy](https://github.com/stackql/stackql-deploy).

## Repository layout

| Path | Contents |
|------|----------|
| `src/main.rs`, `src/app.rs` | Thin CLI entry point (clap) over the library crate |
| `src/lib.rs` | Library target so tests can drive commands in process. **Not a public API** — layout and signatures may change without notice |
| `src/commands/` | One module per subcommand (`build`, `test`, `teardown`, `init`, `info`, `shell`, `start-server`, `stop-server`, `upgrade`) |
| `src/core/` | Config, env loading, errors, secrets and log redaction, templating helpers |
| `src/resource/` | Manifest parsing and validation |
| `src/template/` | Tera engine, custom filters, render context |
| `src/utils/` | stackql binary download and server management, pgwire client, query execution, display and logging |
| `tests/` | Integration tests — see [tests/README.md](tests/README.md) |
| `ci-scripts/` | Bash scripts used by CI and for local checks |
| `examples/`, `template-hub/` | Example stacks per provider and the `init` scaffolding templates |
| `website/` | Docusaurus site for stackql-deploy.io; user docs are under `website/docs/` |
| `docs/` | Maintainer docs: flow diagrams and the [release process](docs/release-process.md) |
| `build.rs` | Creates an empty `contributors.csv` placeholder when absent (CI injects the real one) |

## Build, lint, and test

CI (`.github/workflows/ci.yml`) runs `ci-scripts/lint.sh` and
`ci-scripts/test.sh` on every pull request to `main`; both must pass. Run the
same checks locally before pushing:

```sh
bash ci-scripts/format.sh      # cargo fmt --all
bash ci-scripts/lint.sh        # cargo clippy -- -D warnings && cargo fmt -- --check
bash ci-scripts/test.sh        # cargo test --lib && cargo test --test '*'
cargo check --release          # fast compile check, also run in CI
cargo build --release          # binary at target/release/stackql-deploy
```

- Clippy runs with `-D warnings`; any new warning fails CI. Fix it rather than
  allowing it.
- Formatting is checked by `cargo fmt -- --check`; run `cargo fmt --all` before
  committing.
- The scripts are bash. On Windows run them from Git Bash or another shell
  with `sh` on `PATH`; the underlying `cargo` commands work from any shell.
- `cargo doc --no-deps` (`ci-scripts/doc.sh`) must stay green. Doctests are
  deliberately disabled in `Cargo.toml` because module-level doc comments
  contain illustrative, non-compilable snippets; do not turn them back on.

### Tests

There are three layers, all driven by `cargo test`:

| Layer | Where | Needs | How to run |
|-------|-------|-------|------------|
| Unit | `#[cfg(test)]` modules in `src/` | nothing | `cargo test --lib` |
| Mock integration | `tests/build.rs`, `tests/teardown.rs`, `tests/test_command.rs`, `tests/fixtures/` | nothing — in-process mock stackql server | `cargo test --test '*'` |
| Live | `tests/live.rs`, `tests/live_stacks/` (`#[ignore]`d) | stackql binary, AWS and GitHub credentials | `bash ci-scripts/integration-test.sh` |

When you change behaviour in a command flow, add or update a mock integration
test that asserts on the exact SQL sent; that is the primary regression suite
and it needs no network or credentials. Live tests run in the
`Integration Tests` workflow on pull requests; they use free resources only and
must tear down everything they create. Details, including how to add a live
stack, are in [tests/README.md](tests/README.md).

Do not run the live tests unless you have been given credentials for them.

## Coding conventions

- Rust 2021 edition, `rust-version = "1.75"` in `Cargo.toml`. Do not use
  features newer than the MSRV without bumping it deliberately.
- Keep `main.rs` thin; put logic in the library modules so it is testable
  in process.
- Application errors are variants of `AppError` in `src/error.rs`; add a
  variant rather than returning bare strings. `src/core/errors.rs` holds the
  pattern list that classifies a stackql error as fatal (network, auth,
  planner/compiler errors) versus a provider error that retry loops and
  `--on-failure ignore` may tolerate. Keep that distinction when adding
  error handling; the 2.1.1 and 2.2.0 entries in `CHANGELOG.md` describe the
  intended behaviour.
- Anything that can hold a secret (`protected` globals, props, and exports)
  must be masked in every log path, including `--dry-run`, `--show-queries`,
  `DEBUG` logging, and `RETURNING *` response logging. Redaction is
  value-based at the logger sink (`src/core/secrets.rs`); register new
  sensitive values there rather than masking ad hoc.
- Only comment code that needs clarification. Module-level `//!` docs
  describing a module's role are welcome.
- Match the surrounding style; do not reformat or rename unrelated code.

## Documentation and changelog

- Record every user-visible change in `CHANGELOG.md` under the version being
  released, in a `### Features` or `### Fixes` list. Entries are full prose
  sentences describing the behaviour before and after, and link GitHub issues
  as `([#56](https://github.com/stackql/stackql-deploy-rs/issues/56))`.
- When a CLI flag, manifest field, anchor, or template filter changes, update
  the matching page under `website/docs/` (`cli-reference/`,
  `manifest_fields/`, `resource-query-files.md`, `template-filters.md`) and,
  if the README shows it, the README.
- `docs/` is for maintainer documentation (diagrams, release process), not
  user docs.

## Git workflow and releases

- Work on a feature or fix branch and open a pull request against `main`.
  Never commit directly to `main` and never force-push it.
- Releases are cut by bumping `version` in `Cargo.toml` (commit `Cargo.lock`
  too) and pushing an annotated `vX.Y.Z` tag, which triggers
  `.github/workflows/release.yml` to build six platform binaries, create the
  GitHub Release, and publish to crates.io. Do not bump the version or create
  tags unless asked; the full procedure is in
  [docs/release-process.md](docs/release-process.md).

## Secrets and generated files

- Never commit credentials. Cloud and GitHub credentials are supplied through
  the environment or a gitignored `.env` file; do not read `.env`, `.env.*`,
  `~/.aws`, or `~/.ssh` and do not echo their contents into logs, tests, or
  fixtures.
- The following are generated or local-only and gitignored; do not commit
  them or depend on their presence: `target/`, `contributors.csv`,
  `.stackql/`, `stackql.log`, `.stackql-deploy-exports`, `stackql-deploy`
  binaries, `tmp/`, `.claude/`.
- Stack manifests take inputs from `-e KEY=value` flags and `--env-file`, not
  from the process environment; keep that distinction when writing examples
  and tests.
