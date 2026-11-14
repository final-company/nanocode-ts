# final-company/nanocode-ts

Final's fork of [nanocodexyz/nanocode-ts](https://github.com/nanocodexyz/nanocode-ts). The `final`
branch (the default) is upstream `master` with a few commits on top, rebased onto each new upstream
`master`. Final's agent-harness-workers install it from this repository's `build-<short sha>`
releases.

Until 2026-11-13 this fork dropped upstream's `6cbf34b` ("run every session in code mode only") and
cherry-picked every other upstream commit onto `1bffe70`. Upstream's `01232e8` (2026-11-04) made
`config.code_mode` an opt-out again (`code_mode: config.code_mode ?? !state.remote`), and Final's
sessions pass `code_mode: false`, so `final` now follows upstream. The old branch is kept as
`final-cherry-picked`.

## What differs from upstream

- `RuntimeOptions.tools.recover` lets the host answer a host-tool call a runtime reset
  interrupted, with `{ result }`, `{ error }` or `null` to keep the SDK's unknown-outcome answer.
  `src/wasm/tools.rs` implements the Rust SDK's `ToolRecovery` through a `tool_recover` bridge op
  for every non-wire host tool, `src/wasm.rs` installs it on the session's tool sets, and
  `src/AgentRuntime.ts` and `src/Contracts.ts` carry the callback. The SDK sets no timeout on it;
  the host bounds its own wait. `DECISIONS.md` describes it. Final uses it to re-attach exec calls
  across a Durable Object reset, to recover MCP calls, and to ask an interrupted approval again.
- `scripts/wasm.mjs` publishes and downloads WASM builds from this repository, and
  `.github/workflows/release.yml` runs `scripts/release.mjs` on every push to `final` (upstream
  releases by hand).

The Rust SDK is not patched: the bridge only uses its public `ToolRecovery` hook and the
`tool_recovery` tool-set builder. If an upstream SDK change removes either, or upstream makes Code
Mode mandatory again, the fork no longer works by rebasing alone; stop and decide what to do.

## Syncing with upstream

```sh
git remote add upstream https://github.com/nanocodexyz/nanocode-ts.git  # once
git fetch upstream
git checkout final && git pull
git log --oneline --reverse final..upstream/master   # what will come in
git rebase upstream/master
```

Conflicts are in the commits above: keep the `tools.recover` bridge and the release settings, and
take upstream's change otherwise. Read `DECISIONS.md` for what came in, especially entries that say
"Breaks": the SDK migrates session storage forward only, and some changes (the `OneLog` turn log of
2026-11-12) leave sessions stored before them empty.

Then run `npm ci --ignore-scripts`, `npm run check:types`, and push with `--force-with-lease`. The
release workflow publishes `build-<short sha>` with `nanocode-ts-<short sha>.tgz`, building the WASM
first when its sources changed (up to an hour). Every release stays installable after later
force-pushes, because its tag keeps its commit.
