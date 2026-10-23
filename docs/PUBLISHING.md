Current transport and release trigger amendment: [Kit HTTP](KIT-HTTP.md). This supersedes historical stdio/PR-trigger examples below.

# Plugin release preparation and publication

Agents implement and test locally and leave changes unstaged and uncommitted.
Agents must not commit, push, merge, tag, publish or dispatch workflows. The
owner commits/pushes development work and reviews/merges release PRs. GitHub
Actions is authorized to write versions, lockfiles, changelogs and release state,
commit/push `release/update-version`, open/update its PR, publish artifacts and
commit/push verified generated registry metadata. No AI service generates notes.

## Local previews

Run from the repository root (agents also set
`CARGO_HUSKY_DONT_INSTALL_HOOKS=true`):

```text
cargo test --locked -p xtask
cargo xtask prepare
cargo xtask example
cargo xtask example --shared
cargo xtask notes rune-filesystem 0.4.1
cargo xtask validate --output temp/release-notes
```

`prepare` is a read-only Git/history preview. `example` uses a synthetic reviewed
filesystem fix and currently prints the following without writing a version:

```markdown
## rune-filesystem-v0.4.2

- Fixes: report confined file read errors clearly
```

This is an example, not a proposed filesystem release. This tooling change alters
no packaged runtime: filesystem remains 0.4.1, Git/Time remain 0.3.0, and other
plugins retain their versions. Current history has no pending plugin release.
`notes` requires the requested version to match both current manifests; it extracts
the matching plugin CHANGELOG section, including prefixed/versioned headings.
Missing, duplicate, mismatched, empty, placeholder, internal-planning or
version-only notes fail before publication. Historical sections remain untouched.
Separators and global generator footers are excluded; substantive Markdown stays
unchanged. Publication never regenerates prose from history.

Only CI or the owner runs `cargo xtask prepare --apply --summary FILE` on a
working repository. It writes release files and refreshes Cargo.lock; it never
stages, commits or pushes. Unit tests use disposable package fixtures.

## Selection and version rules

Preparation parses reviewed Conventional Commits with `git-conventional` and
uses `semver`/`toml_edit`. Features increment minor, breaking changes increment
major (minor before 1.0), and other semantic types increment patch. Breaking
change descriptions become migration notes. Custom semantic types are accepted.
Invalid commit messages, routine `ci(release)`/version/registry bookkeeping and
format-fix bookkeeping produce no release. Private/placeholder descriptions on
a packaged change fail clearly so the owner can supply public wording.

- A packaged plugin file selects that active workspace plugin only.
- A shared crate change selects its local dependency consumers transitively,
  including feature/target dependencies. PDK selects all 14 active plugins;
  sidecar selects the nine native consumers. Inactive drafts are excluded.
- Markdown, examples, mcp.json client configuration, tests and .env files do not
  change packaged behavior. Docs-only and release-tool-only changes produce no
  plugin bump. A semantic docs/test/build commit that changes a packaged source
  file is eligible.
- Root Cargo.toml or Cargo.lock dependency updates conservatively select all
  consumers. A Cargo.lock change accompanying only xtask/tooling changes does
  not select plugins. Narrow dependency-resolution attribution is not attempted.

One fixed branch/PR is reused. Each run starts from the reviewed develop source,
so another push before merging regenerates the same version from that base.
`.release/prepared.json` records the consumed source head and exact prepared
notes. Merging that PR does not consume the same source changes twice. A later
batch preserves pending unpublished notes and pending releases of other plugins.
Preparation reads published versions from origin/main as well as develop's index,
so a CI registry commit need not first be merged back just for version planning.
Keep origin/main fetched for local previews. Do not manually edit prepared notes
without updating/reviewing release state through preparation.

## Event sequence and owner actions

1. Owner commits/pushes development work to develop. Normal CI runs Rust tooling
   tests, plugin builds/tests, Clippy and formatting. Merge and release commits
   receive the same validation; preparation loops are prevented by consumed state,
   not by skipping validation.
2. Only a trusted, still-current develop push prepares after successful checks.
   Preparation validates generated manifests/lock/notes, tests xtask, checks the
   workspace, runs Clippy/formatting, then creates/updates `release/update-version`
   targeting develop. Both title and automation commit are
   `ci(release): update version`. The PR body lists affected plugins, versions
   and exact generated notes. No releasable changes means no PR.
3. Review the diff and normal CI. The branch push uses `GH_TOKEN`, and the CI push
   filter includes `release/update-version`, so validation of generated files
   runs independently of the PR event. PR API calls use `GITHUB_TOKEN`; GitHub
   may place bot-created PR workflow runs behind **Approve workflows to run**.
   Approve those if the merge UI requests it. Require the `build` check and owner
   review, and do not bypass a failed/pending required check.
4. Owner merges the preparation PR into develop. Merge or squash is supported;
   retain `ci(release): update version` as the squash title. Merging into develop
   alone does not publish. Wait for develop validation, then open develop → main
   and promote with a **merge commit**, preserving source ancestry. Squash/rebase
   promotion is rejected for pending releases. No special main commit-title marker
   is required: the reviewed prepared state and main push gate publication.
5. Main publication validates preparation ancestry, absence of additional
   unprepared packaged changes, matching versions and exact notes before builds.
   Only versions absent from the main registry are selected. WASM payloads or
   all four native platform archives are built, attested and saved as immutable
   `plugin-payload-*` Actions artifacts. One publication job per plugin creates
   `<plugin>-v<version>` at the promotion SHA, with the exact matching changelog
   section, then uploads payloads and SHA-256 sidecars.
6. The dependent registry job verifies actual release tags, bodies, payload
   digests/sizes, sidecar digests and download URLs, saves `registry-evidence`,
   then commits/pushes only registry/index.json to main as
   `chore(registry): publish verified plugin metadata`. **This CI source commit
   and push are intentional and authorized.** It refuses concurrent source
   changes. New versions enter discovery after that push; no second owner registry
   commit is normally needed. Finish/recover this job before another promotion.

Artifacts run in the same workflow through explicit `needs` dependencies. They
do not depend on a release-created event from GITHUB_TOKEN. Registry pushes use
GITHUB_TOKEN and normally do not trigger another workflow; even a manual registry
push has no pending versions and cannot start another plugin publication or
preparation loop. GitHub's event behavior is documented in
[Triggering workflows](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow).

## Authentication and repository settings

- Enable Actions and **Allow GitHub Actions to create and approve pull requests**.
  The workflow creates PRs; it does not approve or merge them.
- `GH_TOKEN`: existing PAT or App token with repository Contents write, allowed
  to push the dedicated release branch and trigger validation. PR API writes use
  the job's GITHUB_TOKEN (Contents/Pull requests write); the PAT does not need
  Pull requests write for PR creation. A token updating workflow files may need
  the appropriate workflow permission; preparation itself changes release files.
- Publication/registry jobs need GITHUB_TOKEN Contents write. Provenance jobs
  need Attestations write and OIDC id-token write. Permit the registry bot to
  push its generated main update under repository rules, or the final job fails
  visibly and requires owner intervention. Keep owner review/promotion protected.
- Allow merge commits for main promotion. Require normal CI and owner review on
  the preparation/promotion PRs. Allow the preparation action's fixed branch to
  be updated. No secret value is printed or stored in generated notes/artifacts.

## Retry and registry integrity

Re-run failed publication/registry jobs in the **original** run using its source
SHA and retained payload artifacts. Identical assets are accepted only when
GitHub digest and size match. A new build may change archive timestamps; never
clobber published assets to make a rebuilt archive pass. Only explicit
already-exists races are rechecked. 403/500/network errors, missing assets,
changed notes, missing digests and wrong source tags fail. Do not republish
historical generic release bodies with this tooling.

The registry retains existing URL/native target-string shapes and previous
versions. Its additive `checksums` map records each actual payload hash and
sidecar URL. The current Kit reader accepts these URLs but does not enforce
this expected-checksum map; expected-hash enforcement remains Kit-owned.
The Rust command below verifies GitHub and writes a separate candidate/evidence
directory without modifying source or publishing:

```text
cargo xtask registry --repo OWNER/REPOSITORY --target FULL_PROMOTION_SHA --output temp/registry-evidence
```

If registry publication is blocked by branch rules, the owner can review the
saved candidate/evidence and apply/commit the registry update or configure the
bot allowance, then retry. A concurrent main change is never overwritten.
Already-identical registry content makes a retry a no-op.

## Runtime checks and remaining tools

Rust xtask replaces the Python release and MCP smoke clients/tests. CI requires
no Python. Plain test .env files load through `dotenvy` into child commands,
preserving inherited environment precedence, quoting and variable expansion.
Missing ignored .env files are empty configuration; malformed files fail without
printing values. `encrypted:` files and advanced command/default expansion still use dotenvx
(and its existing key/env behavior). CI installs Node/dotenvx only when required.
These capabilities are retained rather than reimplemented; encrypted-key and
command-substitution integration were not exercised locally.

Git/gh remain for history/API access; Actions YAML and third-party Actions remain
CI infrastructure. tar/Compress-Archive remain thin platform packaging steps.
The independent existing install-skill.ts helper is unrelated skill tooling;
it is not in release preparation/publication. No end-user Rust installation is
introduced. Native/WASM Kit smoke commands are in PLUGIN-CONTRACT.md.

## Installation ownership

Tools has no Kit/Infer installer. Their owners maintain `<actual-user-home>/rune`:
rune-kit[.exe] and rune-infer[.exe] at the root, Kit storage tools and plugins
in tools/plugins, Infer home infer and configuration infer/config. Preserve
explicit overrides including RUNE_KIT_DATA_DIR, never fall back to CWD on home
resolution failure, preserve originals until migration validation, refuse data
conflicts, add PATH once, and leave unrelated projects/models intact. Verification
uses explicit payloads and disposable roots/storage and migrates no owner data.
Public MCP remains Kit 0.5.0+ / 2026-07-28; native/WASM calls remain private ABI.
