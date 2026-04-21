# Changelog

## 2.0.7 (2026-04-19)

### Fixes

- Fixed post-deploy failure for `createorupdate` resources whose `exports` anchor is a literal `SELECT` with no `FROM` clause (a supported and common pattern when the values to export are already known and an extra API round-trip is wasteful). Previously the exports query was executed as a statecheck proxy, and a FROM-less result caused the proxy check to report the resource was not in the desired state, aborting the run. With `createorupdate`, the DML is authoritative, so the exports-as-statecheck proxy is now skipped entirely - `exports` still runs to populate the global context for downstream resources.
- Fixed `stackql-deploy upgrade` downloading the stackql binary twice when the binary was missing: the pre-command binary check triggered a download, and the subcommand dispatch then triggered a second download. `upgrade` is now exempt from the pre-command binary check.
- Server-side notices from provider HTTP 4xx/5xx responses are now detected even when stackql wraps them as a generic `a notice level event has occurred` message with the real status code in the `DETAIL:` payload. Previously these escaped the error check and the `create`/`update`/`delete` operation silently appeared to succeed while the post-deploy `exists` check spun through its retries.
- Collapsed duplicate lines within a single notice's `DETAIL:` payload so repeated provider error bodies are printed once.
- Teardown now tolerates resources with unresolved template variables. If an `exists`, `exports`, or `delete` query references a variable that was never populated (because an upstream resource doesn't exist), the resource is treated as already torn down and skipped, instead of aborting the run. Stacks in a half-baked state can now be torn down cleanly.
- During teardown, `RETURNING` clauses are stripped from rendered `delete` DML when `return_vals.delete` is not configured for the resource. Some providers reject `RETURNING *` on `DELETE`, and teardown has no consumer for the returned data unless the manifest explicitly opts in via `return_vals.delete`. When `return_vals.delete` is configured the `RETURNING` clause is preserved and mapped fields are captured as `this.*` with non-fatal warnings if a mapping cannot be satisfied.
- Stale provider notices are no longer re-surfaced on subsequent queries. stackql emits a cumulative `NoticeResponse` on every query that includes every provider notice observed earlier in the session; the pgwire client now tracks each notice line already surfaced and drops byte-identical re-emissions. Dedup is exact-match (no canonicalization) so two distinct provider errors — which always differ in their embedded request/serving IDs — are never conflated. Fixes spurious `create`/`update`/`delete` failures where a 4xx provider response from an earlier `exists` SELECT was attributed to a later DML.

### Features

- When retries are exhausted on a `statecheck`, `exports` proxy, or post-deploy `exists` check, the last rendered query is now logged at `warn` level so the failing SQL is visible without needing `--show-queries` or `--log-level debug`. Pre-create exists checks (which fast-fail by design) stay silent.

## 2.0.6 (2026-03-28)

### Fixes

- Fixed eager rendering of `statecheck` queries that caused hard failures when `this.*` variables were not yet available (e.g. post-create exists re-run fails due to eventual consistency). `statecheck` now uses JIT rendering like `exports`, deferring gracefully when template variables are unresolved.
- When a deferred `statecheck` cannot be rendered post-deploy, the build falls through to `exports`-as-proxy validation or accepts the create/update based on successful execution.
- Applied the same fix to `teardown`, where `statecheck` used as an exists fallback would crash on unresolved variables instead of skipping the resource.
- Fixed `--dry-run` failures for resources that depend on exports from upstream resources. `create` and `update` query rendering now defers gracefully in dry-run mode when upstream exports are unavailable, and placeholder (`<evaluated>`) values are injected for unresolved exports so downstream resources can still render.
- When a post-create exists re-run fails to find a newly created resource (eventual consistency), the exists query is automatically retried using the `statecheck` retry settings if available, giving async providers time to make the resource discoverable.

### Features

- New optional `troubleshoot` IQL anchor for post-failure diagnostics. When a `build` post-deploy check fails or a `teardown` delete cannot be confirmed, a user-defined diagnostic query is automatically rendered and executed, with results logged as pretty-printed JSON. Supports operation-specific variants (`troubleshoot:create`, `troubleshoot:update`, `troubleshoot:delete`) with fallback to a generic `troubleshoot` anchor. Typically used with `return_vals` to capture an async operation handle (e.g. `RequestToken`) from `RETURNING *` and query its status via `{{ this.<field> }}`. See [resource query files documentation](https://stackql-deploy.io/docs/resource-query-files#troubleshoot) for details.
- The `RETURNING *` log message (`storing RETURNING * result...`) is now logged at `debug` level instead of `info`.

## 2.0.5 (2026-03-24)

### Fixes

- Network and authentication errors (DNS failures, 401/403 responses) are now detected early and surfaced as fatal errors instead of being silently retried.
- Unresolved template variables are caught at render time with a clear error message identifying the missing variable and source template.
- `command` type resources now log query output when using `RETURNING` clauses, matching the behavior of `resource` types.
- Stack level exports (`stack_name`, `stack_env`) are now set as scoped environment variables on the host system for use by external tooling.

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
