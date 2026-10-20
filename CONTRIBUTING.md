# Contributing to stackql-deploy

**Note**:

- All contributions to `stackql-deploy`, through any means, are subject to the [license](/LICENSE).
- All contributions to `stackql-deploy` and actions related, through any means, are subject to the [CODE OF CONDUCT](/CODE_OF_CONDUCT.md).
- [AGENTS.md](/AGENTS.md) is the canonical developer guide for this repository: layout, build, lint and test commands, coding conventions, documentation rules, and the git workflow. It is written for AI coding agents but applies equally to people; please read it before starting work.
- This document and the patterns specified herein are influenced by [redis](https://github.com/redis/redis). We thank the maintainers of same, from whom we draw inspiration.

## Contribution flow

Before beginning implementation work, please raise an issue and receive a consensus that work should proceed.

Please raise pull requests (PRs) against `main` from a fork of this repository. Maintainers work on branches in this repository named `feature/<topic>` or `fix/<topic>`; a usefully named branch for the PR source is appreciated either way. The richer the detail and evidence supplied, the easier the review process.

## Before you open a PR

Run the same checks CI runs on every pull request; all of them must pass:

```sh
bash ci-scripts/format.sh   # cargo fmt --all
bash ci-scripts/lint.sh     # cargo clippy -- -D warnings && cargo fmt -- --check
bash ci-scripts/test.sh     # cargo test --lib && cargo test --test '*'
cargo check --release
```

- Clippy runs with `-D warnings`; fix a new warning rather than allowing it.
- The crate is Rust 2021 with the `rust-version` declared in `Cargo.toml`. Do not use newer language or library features without raising the MSRV deliberately.
- When you change behaviour in a command flow, add or update a mock integration test under `tests/` that asserts on the exact SQL sent. These run in-process against a mock stackql server and need no network, binary, or credentials. See [tests/README.md](/tests/README.md).
- The live integration tests (`tests/live.rs`) need AWS and GitHub credentials. They run in the `Integration Tests` workflow for PRs raised from branches in this repository; for PRs from forks the workflow is skipped and a maintainer runs it before merging. Do not run them locally unless you have been given credentials.
- Record every user-visible change in [CHANGELOG.md](/CHANGELOG.md) under the version being released, as a full prose sentence describing the behaviour before and after, linking the GitHub issue.
- When a CLI flag, manifest field, anchor, or template filter changes, update the matching page under `website/docs/` and, if the README shows it, the README.

## Secrets and generated files

Never commit credentials. Cloud and GitHub credentials are supplied through the environment or a gitignored `.env` file; do not echo them into logs, tests, or fixtures. Generated and local-only paths (`target/`, `contributors.csv`, `.stackql/`, `stackql.log`, `.stackql-deploy-exports`, local `stackql-deploy` binaries, `tmp/`) are gitignored; do not commit them or depend on their presence.

## Releases

Releases are cut by maintainers: a version bump in `Cargo.toml` and `Cargo.lock`, then an annotated `vX.Y.Z` tag that triggers the release workflow to build the platform binaries, create the GitHub Release, and publish to crates.io. Do not bump the version or create tags in a contribution PR unless asked. The procedure is in [docs/release-process.md](/docs/release-process.md).

Finally, in respect of all your efforts; we thank you and ask that you please give us a star while you are at it.

Many Thanks,

The StackQL team.
