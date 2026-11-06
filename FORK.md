# final-company/nanocode-ts

Final's fork of [gakonst/nanocode-ts](https://github.com/gakonst/nanocode-ts). The `final` branch
(the default) is upstream without `6cbf34b` ("feat!: run every session in code mode only", #168):
every other upstream commit is cherry-picked onto `1bffe70`, the commit before it, with `-x`, so each
carries `(cherry picked from commit <upstream sha>)`.

Final's agent-harness-workers drive Claude and Codex through `AgentRuntime` with native tools, as
Claude Code and Codex do. Code Mode gives the model only `exec` and `wait`, which breaks that tool
fidelity and does not match a Claude subscription speaking as Claude Code, and upstream has no
opt-out.

## What differs from upstream

- `config.code_mode` exists and defaults to false: a session offers every tool directly unless it
  sets `code_mode: true`. A `hands` session defaults to true, so its cells keep the local
  `LocalCells` fallback upstream relies on; a `remote` session never has local cells.
- Vercel's `Harness.create` keeps `codeMode?: boolean` and `App.create` passes `codeMode: true`; the
  Cloudflare `App.create` keeps `codeMode: false`.
- `AGENTS.md` describes Code Mode as a session's choice.
- `scripts/wasm.mjs` publishes and downloads WASM builds from this repository, and the release
  workflow runs on pushes to `final`.
- `RuntimeOptions.tools.recover` (#2) lets the host answer a host-tool call a runtime reset
  interrupted, with `{ result }`, `{ error }` or `null` to keep the SDK's unknown-outcome answer.
  `src/wasm/tools.rs` implements the Rust SDK's `ToolRecovery` through a `tool_recover` bridge op,
  `src/wasm.rs` installs it on the session's tool sets, and `src/AgentRuntime.ts` and
  `src/Contracts.ts` carry the callback. The SDK sets no timeout on it; the host bounds its own wait.

The Rust SDK (gakonst/nanocode) and this repository's wasm crate still honor `code_mode: false`, and
the Rust SDK itself is not patched: the wasm crate's tool recovery bridge only uses its public
`ToolRecovery` hook. If an upstream change makes the SDK or `src/wasm.rs`
ignore it, the fork no longer works by cherry-picking alone; stop and decide what to do.

## Syncing with upstream

```sh
git remote add upstream https://github.com/gakonst/nanocode-ts.git  # once
git fetch upstream
git checkout final && git pull
last=$(git log --format=%B | sed -n 's/^(cherry picked from commit \(.*\))$/\1/p' | head -1)
git log --oneline --reverse "$last"..upstream/main   # what will come in
git cherry-pick -x "$last"..upstream/main
```

Never merge or rebase onto `upstream/main`: that brings `6cbf34b` back. Resolve conflicts by taking
upstream's change and keeping the differences above. Recurring ones:

- `AGENTS.md`'s Code Mode paragraphs: keep the fork's wording, take upstream's other edits.
- `src/AgentRuntime.ts`: keep `code_mode: !state.remote && (config.code_mode ?? state.hands)`.
- `src/cloudflare/App.ts` and `src/vercel/Harness.ts`: keep the `codeMode` option and its
  `code_mode` pass-through.
- `src/wasm.rs`, `src/wasm/tools.rs`, `src/AgentRuntime.ts` and `src/Contracts.ts`: keep the
  `tools.recover` bridge (`tool_recover`, `tool_recovery(...)` on each tool set).

Watch for upstream commits that assume every session runs cells (new tools reachable only from
cells, prompts written for `exec`), and check the SDK revision bump in `sdk-revision.json` for
Code Mode changes. Then run `npm ci --ignore-scripts`, `npm run prepare`, `npm run check:types` and
`node scripts/measure-node.ts`, which fails unless both providers call the host tool directly.
Push `final`; the release workflow publishes `build-<short sha>` with
`nanocode-ts-<short sha>.tgz`, building the WASM first when its sources changed (up to an hour).
