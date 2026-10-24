# Plugin release preparation and publication

Agents implement/test locally and leave changes unstaged/uncommitted. Agents
must not commit, push, merge, tag, publish or dispatch workflows, and must not
author or patch plugin CHANGELOG entries. CI generates actual release files from
reviewed commits; the owner commits development work and reviews/promotes releases.
GitHub Actions may update release branches and verified registry metadata.
No AI service generates notes.

## Generator and package selection

`cocogitto` is pinned exactly to **7.0.0** in xtask/Cargo.toml/Cargo.lock.
`cog.toml` configures its Conventional Commit parser, semantic version policy,
grouping, links, contributors and Markdown rendering. xtask does not implement
a second bump algorithm or changelog renderer. semver is used only for validation;
toml_edit synchronizes manifests. Generation runs offline without remote API calls.

- `feat` advances minor, including below 1.0; fix/perf/maintenance advance patch.
- Breaking changes take precedence: major after 1.0, minor below 1.0. These
  Cocogitto commit types are explicitly configured; the integration maps pre-1.0 breaking fixes to a minor increment for classification only.
- All valid semantic types are eligible, including docs/refactor/style/test/build/
  ci/chore/revert/custom. Release/version/registry and autoformat bookkeeping,
  invalid commits, placeholders and internal planning labels are excluded.
- A plugin's packaged source selects that plugin. Read-only Git history and
  the repository's dependency-aware path filter select relevant commits.
  Local dependency traversal adds shared crate paths: PDK selects all 14 active
  consumers; sidecar selects the nine native consumers. Draft plugins are excluded.
- Markdown, examples, tests, mcp.json client configuration and .env changes alone
  do not change payloads and produce no plugin release. Root manifest changes
  compare each plugin's inherited workspace dependencies; lock changes compare
  its resolved dependency graph. Unused workspace entries, host-only tooling and
  changes in another plugin's dependency graph do not select unrelated plugins.
- Registry versions and matching `<plugin>-v<version>` tags define each published
  baseline. Fetch tags and origin/main before a local preview. A newly introduced
  plugin uses its reviewed manifest version as the initial version,
  with no fabricated previous tag or compare link.

Generation uses all eligible history since each published tag. A pending version
is recalculated from that published baseline, so another patch does not repeatedly
increment the unpublished version. More significant changes can advance it. Notes
are regenerated from that same history and replace only an unpublished section;
published sections remain unchanged. `.release/prepared.json` records consumed
source history and exact notes. A merged preparation with no new changes produces
no new PR; changes arriving before merge update the same fixed branch/PR.

After writing versions, preparation resolves full offline Cargo metadata to
refresh Cargo.lock before locked validation. `--no-deps` is insufficient here:
it can leave the lockfile stale after workspace package versions change.

New sections use the shared Cocogitto template: dated version headings,
comparison links between actual previous/new tags, emoji groups and linked commits
with author signatures. PR references remain in commit summaries. Breaking-change
body/footer descriptions are retained. Missing historical commit metadata is not invented. Empty groups are omitted. Publication extracts that exact section;
it never regenerates prose from post-merge Git history.

## Local previews

```text
cargo test --locked -p xtask
cargo xtask prepare
cargo xtask example
cargo xtask example --shared
cargo xtask notes rune-filesystem 0.4.1
cargo xtask validate --output temp/release-notes
cargo xtask verify-promotion
```

These commands do not stage/commit/push or publish. `prepare` is read-only unless
CI/owner explicitly supplies `--apply`; agents do not use --apply on this working
repository. Generator tests write only disposable fixtures. No plugin changelog
was directly edited for this tooling repair and no payload version bump is needed.
Synthetic examples are previews, not proposed releases. Exact generated preview
output and local test evidence are under ignored temp/publication-repair/.

## Owner and CI sequence

1. Owner commits/pushes develop. Only develop pushes trigger integration,
   including merge/release commits. PR and release-branch pushes do not duplicate
   the pipeline. Normal builds/tests/lint/format run first.
2. After successful source validation, Actions checks the tested SHA is still
   current, generates versions/changelogs/lock/state, validates generated files
   in that original run and checks develop again before updating the PR. A stale
   preparation fails clearly; the newer tested run must prepare instead.
3. One `release/update-version` PR targets develop, with title and automation
   commit `ci(release): update version`. Its body lists affected packages/versions
   and exact generated notes. No eligible payload changes means no PR.
4. Owner reviews/merges preparation into develop (merge or squash with its
   automation title retained), waits for validation and promotes develop → main
   with a MERGE COMMIT to preserve source ancestry. **No specially typed main
   merge title is required.** Merging into develop alone does not publish.
5. Every main push validates the prepared plugin set. No unpublished manifest
   versions is an explicit successful no-op. Malformed/decreasing versions,
   missing/duplicate prepared state, unconsumed payload changes or mismatched/
   empty/placeholder notes fail with actionable messages. Source ancestry is
   required; eligibility does not depend on the HEAD title or its parent count.
6. Eligible packages build attested WASM payloads or four native archives, saved
   as immutable Actions artifacts. Dependent jobs publish exact version notes,
   payloads and SHA-256 sidecars, then verify actual releases before generating
   registry entries. Jobs use explicit `needs` dependencies, not bot release events.
7. The registry job intentionally commits/pushes only verified registry/index.json
   using GITHUB_TOKEN. Such pushes normally do not start another workflow; even
   an owner registry-only push has no pending versions and validates as a no-op.

## Permissions and recovery

Enable Actions and allow Actions to create PRs. PR API operations use the job's
GITHUB_TOKEN with Contents/Pull requests write. `GH_TOKEN` is a PAT/App token
with Contents write for the fixed preparation branch; PR creation does not require
that token to have Pull requests write. Generated files are checked in the same
preparation run, independent of downstream events. Owner review and successful
integration checks remain required. Publication needs Contents write; attestation
jobs need Attestations write/OIDC. Main branch rules must permit the intended CI
registry update, or an owner must review/apply the saved candidate/evidence.
No secrets are printed or included in generated notes. Settings were not changed.

Re-run failed publication/registry jobs in the original run using its promotion
SHA and retained payload artifacts. Rebuilding archives may change bytes. Existing
assets are accepted only with matching digest/size; changed bytes, wrong notes,
wrong tags, missing assets/digests or 403/500/network errors fail. No --clobber or
blanket error suppression is used. Existing generic historical bodies are not
rewritten. Registry candidates preserve previous versions and URL compatibility,
adding payload hashes and sidecar URLs only after actual asset verification.
An identical registry candidate makes retry a no-op; concurrent changes are refused.

## Installation and HTTP acceptance

Canonical root: `<actual-user-home>/.rune`. Binaries rune-kit[.exe] and
rune-infer[.exe] live directly there; Kit storage/payloads are `.rune/tools` and
`.rune/tools/plugins`; Infer uses `.rune/infer` and `.rune/infer/config`.
Preserve explicit overrides including RUNE_KIT_DATA_DIR. Kit/Infer own migration
from ~/rune and earlier legacy defaults: retain originals until validation,
refuse conflicts, avoid unrelated projects/models and update PATH without duplicates.
Tools adds no migration implementation or installer and moved no user data.

HTTP artifact tests retain the existing native/WASM ABI, schemas, annotations,
read-only ceiling and allowed-root capability. Use disposable storage at
`<fixture>/.rune/tools`, preserving an explicit override; never use owner storage
for acceptance. See KIT-HTTP.md for commands and compatibility requirements.
Plain dotenv configuration remains Rust; encrypted/advanced configuration keeps
its optional dotenvx behavior. Thin platform archive steps and third-party Actions
remain necessary infrastructure. End users need no Rust toolchain.
