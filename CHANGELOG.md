# Changelog

## 2.0.4 (2026-03-18)

### Identifier capture from `exists` queries

The `exists` query can now return a named field (e.g. `vpc_id`) instead of `count`. The returned value is automatically captured as a resource-scoped variable (`{{ this.<field> }}`) and made available to all subsequent queries (`statecheck`, `exports`, `delete`) for that resource. This enables a two-step workflow where `exists` discovers the resource identifier and `statecheck` verifies its properties.

- When `exists` returns `null` or empty for the captured field, the resource is treated as non-existent
- Multiple rows from an `exists` (identifier pattern) or `exports` query is now a fatal error
- After a `create`, the `exists` query is automatically re-run to capture the identifier for use in post-deploy `statecheck` and `exports` queries

### `RETURNING *` identifier capture

When a `create` statement includes `RETURNING *` and the response contains an `Identifier` field, it is automatically injected as `this.identifier` — skipping the post-create `exists` re-run and saving an API call per resource.

### `return_vals` manifest field

New optional `return_vals` field on resources to explicitly map fields from `RETURNING *` responses to resource-scoped variables:

```yaml
return_vals:
  create:
    - Identifier: identifier   # rename pattern
    - ErrorCode                 # direct capture
```

If `return_vals` is specified but the field is missing from the response, the build fails.

### `to_aws_tag_filters` template filter

New AWS-specific Tera filter that converts `global_tags` (list of `Key`/`Value` pairs) to the AWS Resource Groups Tagging API `TagFilters` format:

```sql
AND TagFilters = '{{ global_tags | to_aws_tag_filters }}'
```

### YAML type preservation fix

Fixed an issue where YAML string values that look like numbers (e.g. `IpProtocol: "-1"`) were being coerced to integers during JSON serialization. String types declared in YAML are now preserved through to the rendered query.

### Teardown improvements

- Teardown no longer retries exports queries that return empty results — missing exports are set to `<unknown>` and teardown continues best-effort
- Post-delete existence checks accept the first empty response instead of retrying, reducing teardown time significantly

### AWS starter template updated

The `stackql-deploy init --provider aws` starter template now uses:
- `awscc` (Cloud Control) provider instead of `aws`
- CTE + INNER JOIN exists pattern with `to_aws_tag_filters`
- `AWS_POLICY_EQUAL` for statecheck tag comparison
- `this.<field>` identifier capture pattern
- `RETURNING *` on create statements
- `stackql:stack-name` / `stackql:stack-env` / `stackql:resource-name` tag taxonomy

### AWS VPC Web Server example

Complete rewrite of the `examples/aws/aws-vpc-webserver` stack (renamed from `aws-stack`) using the `awscc` provider exclusively. Includes 10 resources demonstrating all query patterns: tag-based discovery, identifier capture, property-level statechecks, PatchDocument updates, and the `to_aws_tag_filters` filter.

### Patch Document Test example

New `examples/aws/patch-doc-test` example demonstrating the Cloud Control API `UPDATE` workflow with `PatchDocument` — deploy an S3 bucket, modify its versioning config in the manifest, and re-deploy to apply the update.

### Other changes

- Fixed `init` command missing `--env` argument (defaulting to `dev`)
- Added `debug` log import to build command
- Debug logging now shows full `RETURNING *` payloads
- Documentation updates: `resource-query-files.md`, `template-filters.md`, `manifest-file.md`, and AWS template library

## 2.0.0 (2026-03-14)

### Initial Rust Release

This is the first release of **stackql-deploy** as a native Rust binary, replacing the Python implementation.

**Key changes from v1.x (Python):**
- Complete rewrite in Rust — single static binary, no Python runtime required
- Same CLI interface: `build`, `test`, `teardown`, `init`, `info`, `shell`, `upgrade`, `plan`
- Multi-platform binaries: Linux x86_64/ARM64, macOS Apple Silicon/Intel, Windows x86_64
- Available on [crates.io](https://crates.io/crates/stackql-deploy) via `cargo install stackql-deploy`

**The Python package (v1.x) is now archived.** See the [Python package changelog](https://github.com/stackql/stackql-deploy/blob/main/CHANGELOG.md) for the v1.x release history (last Python release: v1.9.4).
