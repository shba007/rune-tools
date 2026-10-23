# Owner-controlled plugin publication

Agents implement and test locally and leave changes uncommitted. Only the owner
stages, commits, pushes, merges, creates tags/releases, or dispatches workflows.
The owner prepares versions and changelogs before committing. No workflow creates
source commits, pushes source branches, or opens automated version PRs.

## Prepare and preview

For a packaged behavior change, update only affected plugins' Cargo.toml and
plugin.toml versions and add a matching section to each plugin's CHANGELOG.md.
If shared PDK/sidecar behavior changes, explicitly identify/version all packaged
consumers. Client examples, documentation and release tooling alone need no
plugin bump. The current tooling change leaves filesystem 0.4.1, git/time 0.3.0
and all other packages unchanged.

New entries describe observable functionality, fixes, compatibility and migration.
Do not put internal planning identifiers in new notes or suggested commit subjects.
A version-bump-only subject is not a substantive description. Historical entries,
tags and published bodies are preserved.

Python 3.11+ is required for this developer/CI tooling (CI pins 3.13), not for
installed plugin users. From the repository root:

```powershell
python scripts/plugin-release.py notes rune-filesystem 0.4.1 --output notes-preview.md
python scripts/plugin-release.py validate --output release-notes
python -m unittest discover -s scripts/tests -v
cargo xtask detect-publishable
```

The first command is a real historical-version preview, not an instruction to
republish filesystem 0.4.1. For a new release, substitute the prepared plugin
name/version. Preview output includes the exact matching version heading and
Markdown body. Repository headings such as `## rune-filesystem-v0.4.1 - 2026-10-23`
are supported, as are plain or bracketed version headings. Section-boundary
separators, a global generator footer and trailing whitespace are omitted;
substantive Markdown is preserved. No notes are derived from Git history.
Missing, duplicate, mismatched, empty, placeholder or version-bump-only notes
fail. Cargo.toml and plugin.toml names/versions must match. The owner reviews the
substantive wording; mechanical validation cannot prove editorial completeness.

## Owner release actions

1. Review the local implementation, proposed versions and note previews. Commit
   and push develop yourself, then review/merge the release to main yourself.
   These are owner actions; agents must stop before them.
2. The owner's push/merge to main triggers `publish.yml`. Its validation step
   checks **all** pending notes before any build or publication. Only versions
   missing from registry/index.json are planned. CI on develop/PRs only checks
   source, formatting, tests and notes; it never fixes/commits source or bumps versions.
3. Build jobs produce the selected WASM cdylib or four native archives. They save
   provenance attestations and immutable `plugin-payload-*` CI artifacts. No
   standalone CLI wrappers or third-party binaries are packaged.
4. One publication job per plugin downloads those saved payloads. It creates a
   release at the exact owner source commit using the matching plugin CHANGELOG
   section as the complete substantive body. It computes SHA-256 from each actual
   payload, uploads `<asset>.sha256`, and verifies GitHub's digest and size for
   both payload and sidecar. No blanket `|| true`, `--clobber`, release-body edit,
   Git source commit or source push is used.
5. After all publications succeed, `registry-handoff` reads the actual releases
   and verifies their source tags, exact notes, expected assets and SHA-256
   evidence. It uploads a generated index, registry.patch, release-evidence.json
   and instructions. It does **not** modify or push repository source.
6. **A second owner action is required for registry discovery.** Download the
   workflow's `registry-handoff` artifact. In an up-to-date main checkout, review
   its evidence and patch, then run these commands yourself (replace the path):

   ```powershell
   git apply --check --ignore-space-change <artifact-directory>/registry.patch
   git apply --ignore-space-change <artifact-directory>/registry.patch
   git diff -- registry/index.json
   ```

   The whitespace flag accommodates Windows checkout line endings. Review the
   complete diff and resolve any conflict without overwriting concurrent data. Commit
   registry/index.json and push main yourself. Finish this handoff before starting
   another release. An empty patch needs no commit. Existing discovery remains
   usable throughout; the new versions become discoverable only after this push.
   Synchronize develop with the approved registry update through the owner workflow.

## Retry and integrity behavior

Re-run failed **publication/handoff jobs** in the original workflow run, retaining
the original source commit and saved build artifacts. Already uploaded assets
are accepted only when SHA-256 and size match. Changed bytes, missing digest
evidence, different existing release notes or a different source tag fail clearly.
Only explicit already-exists races are rechecked; permission, connectivity and
server failures propagate. Never overwrite a published asset to make a retry pass.
Rebuilding archives can change bytes because of timestamps; a new build is not
assumed identical to the retained payload. Historical releases with generic bodies
are not silently rewritten by the new publisher.

The generated index retains existing `url` and native target-to-string mappings
for compatibility, adding a `checksums` map keyed by payload filename, with
`sha256` and checksum-sidecar `url`. Old registry versions and unrelated entries
are preserved. The current Kit registry reader accepts the URLs and computes a
local installed hash, but does **not** enforce this new expected-checksum map;
expected-hash enforcement remains a Kit-owned integration limitation. Release
tooling verifies the GitHub asset digests and sidecars before generating entries.

`cargo xtask update-registry` remains a legacy local URL planner. It does not
verify artifacts/checksums and is no longer used or recommended for publication.
The verified handoff command is:

```text
python scripts/plugin-release.py registry --repo OWNER/REPOSITORY --target FULL_RELEASE_SOURCE_SHA --output registry-handoff
```

That command only reads GitHub and writes the output directory. `notes`,
`validate` and unit tests never publish. `publish` is reserved for the
owner-triggered workflow; agents must not invoke it against GitHub.

## Installation ownership

Tools has no Kit/Infer installer. Their owners maintain the shared default root
`<actual-user-home>/rune`: rune-kit[.exe] and rune-infer[.exe] at the root; Kit
storage in tools, plugin payloads in tools/plugins, Infer home in infer and
configuration in infer/config. Preserve explicit overrides (including Kit's
RUNE_KIT_DATA_DIR), resolve home without a working-directory fallback, add the
shared root to user PATH at most once, preserve originals until validation,
refuse conflicting destinations, and do not move unrelated projects/weights.
Tools verification uses explicit payload paths and disposable roots/storage;
it does not install either product or migrate owner data.
