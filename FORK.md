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
- `RuntimeOptions.tools.recover?(name, signal, context)` answers a tool call that a reset of the
  session's host interrupted. The SDK resumes such a call without its result and, by default, tells
  the model that the outcome is unknown. Final's harness recovers a durable sandbox process by
  `context.call_id`, so a Durable Object reset during a deploy does not lose the result.
  - The SDK calls it only for a call that it recovers, never for `exec` or `wait`. The context has
    no call input.
  - It resolves with a result in the shape that `tools.call` returns, converted the same way,
    including `content` results. `null` keeps the default answer. A rejection also keeps it, and
    the turn continues.
  - `src/wasm/tools.rs` implements the SDK's `ToolRecovery` over the `tool_recover` bridge op, and
    `src/wasm.rs` sets it with `tool_recovery` on the Claude and Codex tool sets. It covers the
    workspace, host, MCP and search tools in each set. The Claude backend reads it through
    `ClaudeTools::recovery` from `install_shared_tools`.
  - The Cloudflare `App.create` takes `tools.recover?(recovery, env, signal)`, with
    `App.HostToolRecovery` (`HostToolCall` without `input`). It rejects while the request
    authority is stopped and counts `host_tool_recoveries`. The Vercel harness does not set it.
  - The SDK has no timeout for recovery. A host that waits on a slow source bounds the wait
    itself, for example with `AbortSignal.timeout`, and rejects to keep the default answer.
  - `node scripts/measure-node.ts` checks it for both providers. It opens a session, stops a turn
    in a host tool call, reopens the session from the same SQLite file and drives the turn again.
    `recover` resolves with the result, with `null` or rejects. The script fails unless the model
    reads the recovered result or the default answer and the reopened session does not call the
    tool again. It passed on 2026-11-04 with WASM fingerprint `971c70a7414895aac8b7` and Node
    26.12.0. Against the WASM build without the change, the recovered-result case fails.

The Rust SDK (gakonst/nanocode) is not patched, and it and this repository's wasm crate still
honor `code_mode: false`. The wasm crate differs from upstream only in the recovery wiring above. If
an upstream change makes the SDK or `src/wasm.rs` ignore `code_mode: false`, the fork no longer
works by cherry-picking alone; stop and decide what to do.

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
- `src/wasm.rs`, `src/wasm/tools.rs`, `src/AgentRuntime.ts`, `src/Contracts.ts`,
  `src/cloudflare/App.ts` and `scripts/measure-node.ts`: keep `tools.recover` and its
  `tool_recovery` wiring. If an SDK revision bump changes `ToolRecovery` or stops reading
  `Tools::recovery`, run `node scripts/measure-node.ts` against a fresh WASM build.

Watch for upstream commits that assume every session runs cells (new tools reachable only from
cells, prompts written for `exec`), and check the SDK revision bump in `sdk-revision.json` for
Code Mode changes. Then run `npm ci --ignore-scripts`, `npm run prepare`, `npm run check:types` and
`node scripts/measure-node.ts`, which fails unless both providers call the host tool directly.
Push `final`; the release workflow publishes `build-<short sha>` with
`nanocode-ts-<short sha>.tgz`, building the WASM first when its sources changed (up to an hour).
