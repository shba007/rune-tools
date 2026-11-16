# Rune Tools — Goal

> This file states direction, not status. What works today is in `README.md`, `AGENTS.md` and `TODO.md`, and the
> code is the source of truth over any document. Decisions recorded here were
> made by the owner on 2026-11-16.
## The whole goal

Rune is a local, Jarvis-like AI harness. You talk or type, or it proposes work
itself; it does the work on your own devices with your own models, tools and
memory; it shows the real result of every step while it works; and it asks
before anything risky.

### Principles

1. **Local first, with a fixed fallback order.** A model runs on the device in
   use when it fits. If it does not fit, it runs on the user's own PC. If it
   does not fit there, it runs in the cloud. If all three fail for that model,
   the next equivalent model is tried in the same order. A cloud-only model
   goes straight to the cloud. Cloud is optional in general and is the expected
   path only for devices too weak to run models (laptop, phone, later IoT).
2. **Strict ownership.** Each concern has exactly one owner (table below).
3. **Three kinds of step, one owner each.** A model call belongs to Infer, a
   tool call to Kit, a screen action to Studio. A tool or model call is
   preferred; a UI Automation element comes next; pixels are last.
4. **Show the thing, not a description.** Every step shows what it actually
   produced (the email text, the table rows, the photos) and decisions are made
   on that content. The view is generated automatically from the schema the
   owner publishes: Kit for tools, Infer for model operations. No model
   generates a layout, and screen captures are not used as a substitute view.
5. **Live and replayable.** Steps appear as they start. The same event stream
   drives the live view and playback. The pill shows a glance; a workbench
   window shows and decides.
6. **Decisions are choices.** Several options, one suggested, each showing its
   outcome on the real content. Not a bare yes or no.
7. **A model per step.** Studio chooses which model a step needs, by the step's
   difficulty and the speed required. Infer decides where that model runs,
   following principle 1.
8. **Sub-agents when the goal needs them.** The plan shown to the user states
   whether one agent or an orchestrator with several sub-agents will do the
   work. Simple requests use one agent. When sub-agents are used they work in
   parallel wherever their work does not conflict.
9. **Autonomous by proposal.** Rune looks for useful work without being asked,
   by observing through tools (inbox, calendar, files), never by watching the
   screen continuously. By default it only suggests. Preparing work for
   approval is opt-in per area. Content from outside never raises what it may
   do unasked.
10. **Ask before assuming.** When a request lacks something needed to plan, or
    contradicts itself, Rune asks the user directly before planning: few
    questions, each with options and a suggested answer. It does not ask what
    it can look up or safely default, and it does not plan on a guess.
11. **Plans can grow.** An approved plan is a starting point. While working, a
    step may be added when a need is discovered, such as looking something up,
    fetching a file or producing something with a model. Added steps are shown
    as added and pass the same safety gate as planned ones.
12. **Shared materials with provenance.** Everything gathered or produced for a
    goal is kept in one place that every step and sub-agent can reuse, so the
    same item is not fetched or produced twice. Each item records where it came
    from: its source and licence if fetched, the model if generated. Claims
    taken from outside carry their source. Requests that leave the machine,
    such as a web search, must not contain sensitive data.
13. **Drafts, not restarts.** A result is kept as parts that depend on each
    other. The user can give feedback on any part; only what depends on that
    part is redone, and the outcome is a new draft. Earlier drafts are kept for
    comparison and return.
14. **Reuse before building.** Existing engines, MCP servers, research and
    projects are preferred over new code.
15. **Small cycles.** Changes ship as small, deployable steps ordered by the
    80/20 rule.
16. **Safe by default.** Risky actions need confirmation, everything is
    audited, and sensitive data classes stay on the user's own hardware.
17. **Scope for now.** Windows, Linux and Android. Both a personal tool and a
    public product.
18. **A non-goal is not a removal order.** Code that falls outside these goals
    stays as it is. Nothing is removed, disabled or rewritten because it is a
    non-goal unless the owner explicitly approves that removal.

### Ownership

| Repo | Owns |
| --- | --- |
| **rune-studio** | The agent runtime and orchestrator, the user interface, the safety gate, screen actions, voice, the run trace, and the bridge that lets outside agents drive the screen |
| **rune-infer** | Every model and engine, including specialist operations (OCR, detection, generation, speech), and where a model runs: device, home PC or cloud |
| **rune-kit** | Every MCP server the harness uses (first-party plugins, third-party stdio and HTTP servers), skills, the sandbox, and shared external binaries such as ffmpeg |
| **rune-tools** | First-party plugins for Kit |
| **rune-lake** | Memory: the media catalog, long-term memory and the encrypted life store |
| **rune-agent** | Sub-agents (personalities) and their workflows: a prompt, skills, MCP servers and a workflow each |
| **rune-model** | The model weight store, nothing more |
## This repo's goal

Tools holds the plugins Rune writes itself, and the registry Kit installs from.
The aim is a small set that is dependable, clear to a small local model, safe
to run, and that returns results Studio can show without extra work.

### Current goals

1. **Output schemas on every tool.** Each tool declares what it returns, with
   display hints where the plain schema is ambiguous, so its result renders
   automatically.
2. **Describe before doing.** Tools that change something can report the change
   first, so the user decides on the real content and can undo.
3. **Fewer, better core tools.** Files, web fetch, browser, git, email and time
   carry most assistant work; their descriptions and schemas must be short and
   unambiguous.
4. **Host instead of write.** Once Kit hosts third-party stdio servers, a new
   first-party plugin is justified only when no dependable existing server does
   the job or it needs Kit's sandbox.
5. **Memory goes through Lake.** `rune-memory` becomes a thin surface over Lake
   or is replaced by Lake's own tools.
6. **A RAW development tool**, taken from Studio's Portfolio flow.
7. **Sources on fetched content.** Fetch, search and download tools return the
   source and, where known, the licence of what they bring in.

### Not goals

- Wrapping Infer's model operations as plugins.
- Running plugins (Kit) or storing memory (Lake).
- Private single-customer integrations in the public registry.
- New work on `rune-sequential-thinking`, `rune-figma`, `rune-slides` or the
  media download tools.

Per principle 18, existing plugins in these areas are not to be removed without
the owner's explicit permission.

### Where the code is today

- Seventeen plugin folders in Kit's own format: five WebAssembly plugins and
  native ones; print, scan and ssh are drafts.
- A plugin development kit, a sidecar runner, contract tests and a build tool.
- `rune-memory` keeps its knowledge graph in a local JSON file.

### Open questions

- Which current plugins are used enough to keep maintaining.
- Whether the registry also lists vetted third-party stdio servers.
