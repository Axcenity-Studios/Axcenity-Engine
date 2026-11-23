# Prioritized Remediation Plan (`8f4c2420`)

Confirmed defects (**Part I**) are strictly separated from engineering recommendations and strategic opportunities (**Part II**).

---

# Part I — Confirmed Defects (Security, Correctness, Stability, Build Health)

## P0 — Critical Correctness, Security, Stability, or Data-Loss

### REM-P0-01 — Broken `axcenity-project` Test Compilation on `main`
- **Subsystem:** Project & Serialization (`crates/project`)
- **Evidence:** `crates/project/src/tests.rs:655-656` fails `cargo test --workspace` with `E0063` (missing `pitch`, `wait_until_done`) and `E0308` (`String` vs `ObjectRef`) in `BlockKind::PlaySound`.
- **Severity:** P0
- **User/Developer Impact:** Blocks running `cargo test --workspace` and `cargo clippy --workspace --all-targets`.
- **Root Cause:** Commit `8f4c2420` updated `BlockKind::PlaySound` in `blocks-runtime` without updating `project/src/tests.rs`; local workspace test was skipped.
- **Recommended Solution:** Update `BlockKind::PlaySound` struct literal in `crates/project/src/tests.rs:655-656` to supply `target: ObjectRef`, `pitch`, and `wait_until_done`.
- **Dependencies:** None.
- **Acceptance Criteria:** `cargo test --locked -p axcenity-project` compiles and passes 100%.
- **Required Tests:** Existing 248 `axcenity-project` tests pass.
- **Potential Regressions:** None (test-only fix).
- **Estimated Complexity:** Trivial (< 1 hour).

### REM-P0-02 — Two Failing `axcenity-script` Unit Tests & Two `axcenity-editor` Clippy Errors
- **Subsystem:** Scripting (`crates/script`) & Editor (`crates/editor`)
- **Evidence:** `crates/script/src/definitions/tests.rs:85` (`PROPERTY_DOCS` drift from `core/src/class.rs`); `crates/script/src/events_tests.rs:90` (`Speed is not a property of Part`); `crates/editor/src/animation/controller_editor/inspector.rs:634` (`collapsible_if`); `crates/editor/src/profiler.rs:979` (`type_complexity`).
- **Severity:** P0
- **User/Developer Impact:** Trunk fails both `cargo test` and `cargo clippy --workspace --all-targets -- -D warnings`.
- **Root Cause:** Recent commits (`788fc3f3`, `dc487f0d`, `8f4c2420`) added `core` properties and tightened `Part` property validation / editor code without running full local checks.
- **Recommended Solution:** Add missing property descriptions to `PROPERTY_DOCS`, fix `events_tests.rs` (use a valid `Part` property or verify intended behavior), collapse `if` in `inspector.rs:634`, and introduce a `type` alias in `profiler.rs:979`.
- **Dependencies:** REM-P0-01 (for full workspace green).
- **Acceptance Criteria:** `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --locked -p axcenity-script` pass with 0 errors.
- **Required Tests:** All 421 `axcenity-script` tests pass.
- **Potential Regressions:** Confirm `GetPropertyChangedSignal` error behavior matches Roblox-familiar contract.
- **Estimated Complexity:** Low (1–2 hours).

### REM-P0-03 — Broken Manual CI Workflow (`.github/workflows/ci.yml`)
- **Subsystem:** Build & CI (`.github/workflows/`)
- **Evidence:** `.github/workflows/ci.yml:69` invokes `cargo clippy -p axcenity-editor --all-targets --features hot-patch --locked -- -D warnings`; `hot-patch` was removed in ADR-0023 (`crates/editor/Cargo.toml:79-85`).
- **Severity:** P0
- **User/Developer Impact:** Manual CI dispatch fails unconditionally; zero working automated verification gate.
- **Root Cause:** `ci.yml` was not updated when `subsecond`/`dx` hot-patching was removed.
- **Recommended Solution:** Remove the stale `--features hot-patch` clippy step and `subsecond` grep reference; add a `scripts/check-local.sh` / `.ps1` pre-push runner enforcing `fmt`, `clippy -- -D warnings`, `test --workspace`, and `check-wiki.py`.
- **Dependencies:** REM-P0-01, REM-P0-02.
- **Acceptance Criteria:** `scripts/check-workflows.py` passes; manual CI dispatch succeeds cleanly.
- **Required Tests:** Local execution of every command in `ci.yml`.
- **Potential Regressions:** None.
- **Estimated Complexity:** Trivial (< 1 hour).

### REM-P0-04 — Collaborate Client Arbitrary File Write / Delete via Host Path Traversal
- **Subsystem:** Project Collaboration (`crates/collab`)
- **Evidence:** `crates/collab/src/client.rs:1119-1131` assigns `ToClient::Welcome { files }` without calling `files::check_path`; `crates/collab/src/mirror.rs:282-319` executes `self.root.join(path)` for `create_dir_all`, file write, and `fs::remove_file`.
- **Severity:** P0 (Critical Security — Arbitrary File Write/Delete & RCE via `plugins/*.lua`)
- **User/Developer Impact:** Joining a malicious or compromised Collaborate session can overwrite `~/.bashrc`, SSH keys, or drop auto-executing `plugins/evil.lua` into the workspace.
- **Root Cause:** Path validation (`files::check_path`) was enforced only on host ingress, trusting host-originated manifests implicitly.
- **Recommended Solution:** Enforce `files::check_path` on **every** file path in `ToClient::Welcome`, `ToClient::FileChanged`, and inside `Mirror::take_theirs` / `Mirror::sync` (defense-in-depth); reject and disconnect on invalid paths; explicitly block `plugins/` writes unless user-approved.
- **Dependencies:** None.
- **Acceptance Criteria:** Any absolute path, `..` component, backslash traversal, symlink escape, or NUL byte from host is rejected before touching disk.
- **Required Tests:** Adversarial host→client integration tests in `crates/collab` sending `../../escape`, `/tmp/pwn`, `a\..\b`, and `plugins/evil.lua`.
- **Potential Regressions:** None for valid relative project paths.
- **Estimated Complexity:** Low (2–4 hours).

### REM-P0-05 — Auto-Execution of Untrusted Project Plugins with `files` Permission
- **Subsystem:** Plugin Architecture (`crates/editor`, `crates/plugins`)
- **Evidence:** `crates/editor/src/app/plugins.rs:66-121` auto-starts every `<project>/plugins/*/init.lua` on project open unless listed in `saved.disabled`, granting `ReadProjectFile`/`WriteProjectFile` (`crates/plugins/src/manifest.rs:26-28`) with zero user consent.
- **Severity:** P0 (Critical Security — Untrusted Code Execution on Project Open)
- **User/Developer Impact:** Opening a downloaded or Collaborate-synced project immediately runs arbitrary Lua with full project read/write privileges.
- **Root Cause:** Opt-out (`disabled`) trust model instead of opt-in (`trusted`) per project/plugin hash.
- **Recommended Solution:** Switch project-local plugins to an **opt-in trust model** keyed by `(project_id/path, plugin_id, content_hash)` stored in user-local editor config (never inside the project folder); display a clear trust/permission prompt showing requested permissions (`files`, etc.) before executing any untrusted project plugin.
- **Dependencies:** REM-P0-04.
- **Acceptance Criteria:** Untrusted project plugins do not execute on project open; explicit user approval persists per content hash; modifying a plugin invalidates approval.
- **Required Tests:** Unit/integration tests verifying untrusted project plugins stay blocked until approved and re-prompt if `init.lua` or `plugin.json` changes.
- **Potential Regressions:** Existing users with local project plugins will see a one-time trust prompt.
- **Estimated Complexity:** Medium (1–2 days).

### REM-P0-06 — Unauthenticated Multiplayer `UserId` Spoofing -> Remote Console Execution
- **Subsystem:** Multiplayer & Networking (`crates/net`)
- **Evidence:** `crates/net/src/session.rs:352-377` accepts `user_id` verbatim from client `Message::Hello`; `crates/net/src/game.rs:1667-1684` authorizes `SessionEvent::Console` against `ConsoleService.AllowedUserIds` using that spoofed `UserId`.
- **Severity:** P0 (Critical Security — Remote Admin Privilege Escalation)
- **User/Developer Impact:** Any modified client can impersonate a server admin's `UserId` and execute arbitrary server console commands.
- **Root Cause:** `UserId` was treated as an identity claim without cryptographic authentication.
- **Recommended Solution:** Separate display/session `UserId` (server-assigned sequential or cryptographic Ed25519 public-key hash) from admin authorization. For dedicated/hosted servers without online accounts, authorize `ConsoleService` via Ed25519 challenge-response against configured admin public keys (reusing `collab::identity` / `ring` Ed25519) or a dedicated server-side admin token over the TLS token port — never a client-asserted integer `UserId`.
- **Dependencies:** None.
- **Acceptance Criteria:** A client sending an arbitrary `Hello.user_id` cannot gain `ConsoleService` admin access; server assigns `UserId` or verifies Ed25519 proof.
- **Required Tests:** Adversarial client test attempting `UserId` spoofing against `AllowedUserIds` and verifying rejection.
- **Potential Regressions:** Update `docs/developer-console.md` and `docs/architecture/networking.md` for admin key configuration.
- **Estimated Complexity:** Medium (1–2 days).

### REM-P0-07 — SSL-Stripping / Plaintext Password Downgrade on First Multiplayer Connect
- **Subsystem:** Multiplayer & Networking (`crates/net`)
- **Evidence:** `crates/net/src/transport.rs:546-550, 642-645` (`Trust::Remember`) sets `fallback = !is_known(...)`; on `FetchError::NoTokenPort`, silently falls back to `unsecured(server)` UDP and sends `Message::Hello` (with server password) in plaintext JSON (`handshake.rs:200-202`).
- **Severity:** P0 (High Security — Credential Disclosure via Active MITM)
- **User/Developer Impact:** An active network attacker blocking TCP port X forces clients to transmit the server password in cleartext.
- **Root Cause:** Convenience fallback for LAN/unsecured servers applied even when a password is supplied or remote host is targeted.
- **Recommended Solution:** Never downgrade silently to plaintext when a password is set or when connecting to non-loopback/non-LAN hosts unless the user explicitly passes `--insecure` or confirms an explicit UI warning; use constant-time comparison (`ring::constant_time` or byte-XOR fold) for passwords (`handshake.rs:356`).
- **Dependencies:** None.
- **Acceptance Criteria:** Blocking the TLS token port causes connection failure (not plaintext fallback) when a password is present or TLS is required.
- **Required Tests:** Test simulating closed/blocked TCP token port with password set -> asserts zero cleartext `Hello` sent and returns typed `TlsRequired` error.
- **Potential Regressions:** Unsecured LAN servers without passwords must explicitly opt into insecure LAN mode.
- **Estimated Complexity:** Low (4–6 hours).

### REM-P0-08 — `crates/repo` GitHub Token Exfiltration, Git Option Injection & Binary Corruption
- **Subsystem:** Collaborate Git Sync (`crates/repo`)
- **Evidence:** (1) `crates/repo/src/git.rs:29, 278-284`: `TOKEN_HELPER` ignores `host=` stdin and leaks GitHub OAuth token to any HTTPS remote/redirect. (2) `git.rs:422-654`: `ls-remote`/`fetch`/`push`/`merge` omit `"--"` and allow leading `-` URLs (`--upload-pack=` RCE). (3) `git.rs:140, 722-726`: `Git::show` decodes stdout via `from_utf8_lossy`, corrupting binary files. (4) `credentials.rs:58-65`: newline injection in `git credential` stdin.
- **Severity:** P0 (Critical Security + Data Corruption)
- **User/Developer Impact:** OAuth token theft, arbitrary command execution via crafted repo URL, and silent corruption of binary assets (`.glb`, `.png`) read via `Git::show`.
- **Root Cause:** Shell credential helper didn't parse stdin `host=`; CLI args weren't delimited with `"--"`; `SystemRunner` forced UTF-8 decoding on raw command output.
- **Recommended Solution:** Scope `TOKEN_HELPER` strictly to `host=github.com` (parsing stdin key/value lines); validate all remote URLs/refs reject leading `-` and pass `"--"` before positional args; return raw `Vec<u8>` in `SystemRunner::run_bytes` for `Git::show`; reject `\r`/`\n` in `credentials.rs`.
- **Dependencies:** None.
- **Acceptance Criteria:** Token helper emits nothing for non-`github.com` hosts; `--upload-pack=id` URLs are rejected; binary `.glb` round-trips byte-identically through `Git::show`.
- **Required Tests:** Unit tests for non-GitHub host token suppression, leading-dash URL/ref rejection, newline rejection in `credentials.rs`, and binary byte-exact `Git::show`.
- **Potential Regressions:** Enterprise GitHub hosts would require explicit configured host allowlist.
- **Estimated Complexity:** Medium (1 day).

### REM-P0-09 — Live MCP TCP Server Privilege Self-Declaration & Weak Endpoint Security
- **Subsystem:** Developer Tooling / MCP (`crates/editor/src/app/mcp/`, `crates/mcp`)
- **Evidence:** `crates/editor/src/app/mcp/live.rs:84-103, 230-231, 472-498` writes `.axcenity/mcp-endpoint.json` with default umask (`0644`) and non-CSPRNG SipHash token, and trusts client-supplied `Hello.mode` (`Mode::Full`) and `Hello.allow_import` (`["/"]`). `crates/mcp/src/safety.rs:103, 124-139` has unbounded `Confirms::issued`.
- **Severity:** P0 (High Security — Local Privilege Escalation / Import Sandbox Bypass)
- **User/Developer Impact:** Any local user/process on multi-user Linux can read the endpoint file and send `Mode::Full` + `allow_import: ["/"]` to import arbitrary filesystem files or bypass destructive-action confirmations.
- **Root Cause:** Client `Hello` frame was trusted to declare policy instead of capping against server-side policy; `std::fs::write` used default permissions.
- **Recommended Solution:** Generate endpoint token via `ring::rand::SystemRandom` (32 bytes hex); create `.axcenity/mcp-endpoint.json` with mode `0600` on Unix (ACL restricted on Windows); enforce server-side maximum `Mode` and server-side `allow_import` allowlist (client can only request a subset, never escalate); cap `Confirms::issued` (e.g., 256 active tokens).
- **Dependencies:** None.
- **Acceptance Criteria:** Client requesting `Mode::Full` or `allow_import: ["/"]` when editor allows `Standard` / `[]` is clamped or rejected; endpoint file is `0600` with CSPRNG token.
- **Required Tests:** Protocol tests in `crates/mcp` & `editor/src/app/mcp` verifying privilege clamping, file permissions, and confirm-queue cap.
- **Potential Regressions:** External MCP CLI clients requesting elevated modes must configure them on the editor side.
- **Estimated Complexity:** Low-Medium (4–8 hours).

### REM-P0-10 — `Arena::insert_at` Unbounded Allocation DoS on Untrusted Replica IDs
- **Subsystem:** Core Runtime (`crates/core`)
- **Evidence:** `crates/core/src/arena.rs:187-197` resizes `self.slots` to `id.index as usize + 1` with no upper bound below `LOCAL_BASE` (`2^31`), allowing a single crafted `InstanceId` to allocate ~32+ GiB and panic/OOM.
- **Severity:** P0 (Stability / Remote DoS)
- **User/Developer Impact:** Corrupted project/save or malicious server/peer crashes client or host via OOM.
- **Root Cause:** Direct vector indexing assumed dense server IDs without enforcing a maximum slot ceiling or sparse fallback.
- **Recommended Solution:** Enforce a configurable hard ceiling (e.g. `MAX_REPLICA_SLOTS = 2_000_000`, or reject jumps > `MAX_SPARSE_GAP` from untrusted ingress) and return `Result<(), ArenaError>` from `insert_at` instead of unconditionally resizing. Also replace O(N) `self.free.retain` (`arena.rs:204`) with O(1) slot-state tracking.
- **Dependencies:** None.
- **Acceptance Criteria:** `insert_at` with `index = (1 << 31) - 1` returns `Err` cleanly with zero huge allocation.
- **Required Tests:** Unit tests in `crates/core` for boundary IDs (`LOCAL_BASE - 1`, `u32::MAX`) and large sparse jumps.
- **Potential Regressions:** Audit all callers of `insert_at` (`net`, `collab`, `save`, `project`) to handle `Result`.
- **Estimated Complexity:** Medium (1 day).

### REM-P0-11 — `crates/script/src/debugger.rs` `unsafe` C-Stack Capacity & Re-Entrancy Gaps
- **Subsystem:** Scripting (`crates/script`)
- **Evidence:** `crates/script/src/debugger.rs:1244-1289` calls `lua_getinfo`, `lua_getfenv`, and `lua_xmove` after `lua_checkstack` failure paths without ensuring stack capacity; `on_step` -> `describe` invokes `tostring.call` (`1439`) re-entrantly inside `step_hook` before `lua_yield`; `describe_table` (`1450-1469`) scans unbounded tables; `lua_singlestep` fires on every opcode even with 0 breakpoints (`1131-1165`).
- **Severity:** P0 (Memory Safety / UB in `unsafe` FFI)
- **User/Developer Impact:** Potential C-stack overflow / UB when inspecting deep frames or large tables in the debugger; unnecessary VM slowdown when debugger is attached with no breakpoints.
- **Root Cause:** Missing `lua_checkstack` guards before unconditional C API pushes; eager `__tostring` invocation inside hook.
- **Recommended Solution:** Guard every `lua_getinfo`/`lua_getfenv`/`lua_xmove` with explicit `lua_checkstack` checks (aborting frame read cleanly on failure); avoid re-entrant Lua `tostring.call` inside `step_hook` (format primitives/userdata type names in Rust); cap `describe_table` iteration; disable `lua_singlestep` when breakpoints & step mode are inactive.
- **Dependencies:** None.
- **Acceptance Criteria:** Zero unchecked FFI stack pushes; deep-stack and 100k-entry table inspection tests complete in <5ms without UB.
- **Required Tests:** Debugger stress tests (deep call stack, 100k-key table, userdata with throwing `__tostring`).
- **Potential Regressions:** Custom Lua `__tostring` previews in debugger hover will show type/raw representation instead of running arbitrary Lua during pause.
- **Estimated Complexity:** Medium (1–2 days).

### REM-P0-12 — Non-Finite Floats (`NaN`/`Inf`) Permanently Corrupt Save Files
- **Subsystem:** Saves (`crates/save`)
- **Evidence:** `crates/save/src/state.rs:116-180` serializes `Value::Number`/`Vector3`/`CFrame`/`Color3` via `serde_json::json!()`, turning `NaN`/`Inf` into JSON `null`, which `from_json` rejects on load (`SaveError::Damaged`).
- **Severity:** P0 (Data Loss)
- **User/Developer Impact:** A single physics NaN or script `0/0` in any saved property silently writes a corrupted save slot that can never be loaded again.
- **Root Cause:** `serde_json` silently maps non-finite `f32`/`f64` to `null`.
- **Recommended Solution:** Validate finiteness in `to_json` (either return a typed `SaveError::NonFiniteValue` before writing or sanitize with logged warning per policy) and run a round-trip validation check before atomic rename.
- **Dependencies:** None.
- **Acceptance Criteria:** Saving a `World` containing `f64::NAN` / `f32::INFINITY` never produces an unreadable save file.
- **Required Tests:** Round-trip unit tests in `crates/save/src/tests.rs` injecting `NaN`, `+Inf`, `-Inf` into `Number`, `Vector3`, `CFrame`, `Color3`.
- **Potential Regressions:** None.
- **Estimated Complexity:** Low (2–4 hours).

### REM-P0-13 — `axcenity-relay` Unsynchronized Concurrent `TcpStream` Writes on `game.control`
- **Subsystem:** Multiplayer Relay (`crates/relay`)
- **Evidence:** `crates/relay/src/lib.rs:245-250, 369, 386, 615, 636` clones `game.control` (`TcpStream`) and calls `write_all` concurrently from UDP and TCP threads with no `Mutex`.
- **Severity:** P0 (Concurrency Bug / Protocol Corruption)
- **User/Developer Impact:** Under concurrent joins/hole-punches, interleaved JSON lines corrupt the host's control stream and drop the hosted game.
- **Root Cause:** `TcpStream::try_clone` shares the kernel FD without user-space write serialization.
- **Recommended Solution:** Wrap `game.control` writer in `Arc<Mutex<TcpStream>>` (or a bounded per-game mpsc writer channel) so every framed line is written atomically.
- **Dependencies:** None.
- **Acceptance Criteria:** Concurrent `Punch` + `Open` stress test produces 100% valid, non-interleaved JSON lines.
- **Required Tests:** Multi-threaded concurrency stress test in `crates/relay` sending 1,000 simultaneous `Punch` and `Open` events.
- **Potential Regressions:** Negligible lock contention on control plane.
- **Estimated Complexity:** Low (2–4 hours).

### REM-P0-14 — Flaky Parallel-Race Unit Test in `axcenity-editor-toolbox`
- **Subsystem:** Editor Toolbox (`crates/editor-toolbox`)
- **Evidence:** `axcenity-editor-toolbox::tests::a_package_is_downloaded_once_and_installed_into_each_project` failed under parallel `cargo test` (`task-61`), passed in 0.00s when run alone.
- **Severity:** P0 (Verification Reliability)
- **User/Developer Impact:** Causes intermittent false failures in workspace test runs, eroding trust in local verification.
- **Root Cause:** Shared filesystem path or environment state across parallel test threads.
- **Recommended Solution:** Isolate test temp directories with unique per-test paths (PID + atomic counter) and avoid mutating shared global env/cache state.
- **Dependencies:** None.
- **Acceptance Criteria:** 50 consecutive parallel runs of `cargo test -p axcenity-editor-toolbox` pass 100%.
- **Required Tests:** Repeat-run stress verification.
- **Potential Regressions:** None.
- **Estimated Complexity:** Trivial (< 1 hour).

---

## P1 — Foundational Architecture & Production-Readiness Deficiencies (Confirmed Defects)

### REM-P1-01 — `axcenity-relay` Open-Relay Abuse, UDP `Bind` Reflection & Unmetered TCP Tunnel
- **Subsystem:** Multiplayer Relay (`crates/relay`, `crates/net/src/relayed.rs`)
- **Evidence:** `relay/src/lib.rs:339-387` (unrate-limited UDP `Bind` -> reflection & 64-player slot DoS), `495-539` (unauthenticated 100-game slot exhaustion), `648-668` (unmetered TCP `splice()` tunnel), `278-295` (plaintext host secret); `net/src/relayed.rs:186-245` (uncapped `read_line` & uncapped OS threads per `Open`/`Punch`).
- **Severity:** P1
- **User/Developer Impact:** Public `axcenity-relay` instances can be trivially exhausted, abused as arbitrary TCP tunnels, or used for UDP reflection attacks.
- **Root Cause:** Prototype relay lacked rate limits, byte quotas, and control-plane encryption.
- **Recommended Solution:** Add per-IP rate limits on `Host` and UDP `Bind`; enforce 16 KiB byte cap on token-exchange `splice()`; require optional relay auth key + TLS on host↔relay control plane; cap `relayed.rs` `read_line` (e.g. 64 KiB) and bound `Open`/`Punch` concurrency.
- **Dependencies:** REM-P0-13.
- **Acceptance Criteria:** Relay rejects >16 KiB splices, rate-limits UDP `Bind` & `Host`, and caps host worker concurrency.
- **Required Tests:** Abuse/quota unit tests in `crates/relay` and `crates/net`.
- **Potential Regressions:** Ensure legitimate token exchanges (<8 KiB) remain unaffected.
- **Estimated Complexity:** Medium (2–3 days).

### REM-P1-02 — Four Archive Unpackers Lack Zip-Bomb Bounds & Symlink Checks
- **Subsystem:** Project, Plugins & Toolbox (`crates/project`, `crates/plugins`, `crates/toolbox`)
- **Evidence:** `plugins/src/bundle.rs:148-166`, `project/src/package.rs:342-367`, `project/src/user_templates.rs:315-350` (also lacks `\` check on Linux), `toolbox/src/guard.rs:43-85`; `project/src/packed.rs:27` lacks total archive byte cap.
- **Severity:** P1
- **User/Developer Impact:** Importing a crafted `.axpkg`, `.axtemplate`, `.axcenity-plugin`, or Toolbox zip can exhaust RAM or disk space (zip bomb) or write symlinks.
- **Root Cause:** Each crate wrote its own ad-hoc `zip` loop instead of sharing `updater/src/extract.rs`'s bounded pattern.
- **Recommended Solution:** Enforce per-entry size, total uncompressed byte cap, entry count cap, `is_symlink()` rejection, and `\` rejection across all 5 unpackers via a shared helper or consistent validation contract.
- **Dependencies:** None.
- **Acceptance Criteria:** Crafted 100:1 zip bomb and symlink zip fail cleanly with bounded memory/disk across all 5 formats.
- **Required Tests:** Zip-bomb, symlink, and backslash-traversal tests for `.axpkg`, `.axtemplate`, `.axcenity-plugin`, `.axgame`, and Toolbox zips.
- **Potential Regressions:** Choose generous legitimate ceilings (e.g., 4–8 GiB total for `.axgame`, 512 MiB for `.axpkg`/Toolbox, 32 MiB for plugins).
- **Estimated Complexity:** Low-Medium (1 day).

### REM-P1-03 — `axcenity-server` Missing SIGINT/SIGTERM Graceful Shutdown
- **Subsystem:** Dedicated Server (`crates/server`)
- **Evidence:** `crates/server/src/main.rs:377-415` runs an infinite `loop {}` with no signal handler; `Unpacked::drop`, `Network::disconnect`, `PortMapper::drop`, and `ServerStats` flush never run on Ctrl+C / `systemctl stop`.
- **Severity:** P1
- **User/Developer Impact:** Every server stop leaks `/tmp/axcenity-server-<pid>`, leaves router UPnP port mappings open for 30 minutes, and drops clients without disconnect frames.
- **Root Cause:** No OS signal hook wired into the server loop.
- **Recommended Solution:** Add a cross-platform Ctrl+C / SIGTERM atomic flag (e.g. `ctrlc` or minimal signal handling) breaking the main loop cleanly; also support `AXCENITY_SERVER_PASSWORD` env var (avoiding `/proc/<pid>/cmdline` password exposure) and randomized temp dir names.
- **Dependencies:** None.
- **Acceptance Criteria:** Sending SIGINT/SIGTERM exits cleanly with code 0, removes temp dir, unmaps port, and flushes stats.
- **Required Tests:** Integration test spawning `axcenity-server`, sending SIGINT, and asserting temp dir removal and clean exit.
- **Potential Regressions:** None.
- **Estimated Complexity:** Low (4–6 hours).

### REM-P1-04 — `net` Token Rate-Limit Bypass, `u32` Wraparound Bug & Unvalidated Floats
- **Subsystem:** Multiplayer & Networking (`crates/net`)
- **Evidence:** (1) `secure.rs:413-422`: wrong password returns before `self.recent.allow(...)` (unlimited brute-force). (2) `predict.rs:421, 458`: `BTreeMap<u32, Move>::first_key_value()` breaks `u32` sequence wraparound. (3) `predict.rs:100-121` & `game.rs:1706-1715`: `yaw`/`stick` and `StreamAround.position` lack `is_finite()` checks.
- **Severity:** P1
- **User/Developer Impact:** Unlimited password guessing on token port; broken character prediction after sequence wraparound; `NaN` poisoning of character state or streaming queries.
- **Root Cause:** Ordering oversight in `Issuer::decide`; `BTreeMap` natural ordering used for wrapping sequence numbers; partial input sanitization.
- **Recommended Solution:** Check `self.recent.allow(...)` *before* password verification; order pending moves relative to `last_seq` using `seq_before` (or `VecDeque`); validate `is_finite()` and `[-1, 1]` clamps on `yaw`, `stick`, and `StreamAround.position`; cap `Hello.peer.name` length.
- **Dependencies:** None.
- **Acceptance Criteria:** Brute-force throttled at 30/min; `u32::MAX -> 0` prediction wraparound passes; `NaN`/`Inf` inputs rejected or sanitized.
- **Required Tests:** Unit tests in `crates/net` for rate-limited wrong passwords, `u32::MAX` wraparound, and `NaN` `Move`/`StreamAround` packets.
- **Potential Regressions:** None.
- **Estimated Complexity:** Low (4–6 hours).

### REM-P1-05 — O(N) Non-Broadphase Physics Spatial Queries
- **Subsystem:** Physics (`crates/physics`, `crates/character`, `crates/sim`)
- **Evidence:** `crates/physics/src/query.rs:243-287` linearly scans all Workspace bounding balls and allocates fresh `rapier3d::Collider`s per hit instead of using Rapier's BVH `QueryPipeline`. Amplified by character movement (up to 64 slices) and sound occlusion (up to 48 rays/frame).
- **Severity:** P1 (Foundational Performance)
- **User/Developer Impact:** CPU frame times degrade linearly with part count in large worlds (contributing to the XB10k 5.14 ms vs Unity 3.11 ms gap).
- **Root Cause:** Standalone `query.rs` pre-dated full Rapier scene sync and was never migrated to Rapier's `QueryPipeline`.
- **Recommended Solution:** Maintain and query Rapier's `QueryPipeline` (updated from the existing `RigidBodySet`/`ColliderSet`) for `raycast`, `shapecast`, and `parts_in_*`, falling back to custom checks only where `World` has unsynced edit-mode parts.
- **Dependencies:** None.
- **Acceptance Criteria:** Equivalent query results on all 161 `physics` tests; >=3× speedup on 10,000-part raycast benchmark.
- **Required Tests:** Parity tests comparing BVH vs legacy results across parts, MeshParts, terrain, and collision groups, plus S10k benchmark.
- **Potential Regressions:** Verify edit-mode picking (when physics isn't stepping) still works accurately.
- **Estimated Complexity:** High (3–5 days).

### REM-P1-06 — Unbounded Session RAM Caches (`Assets`, `AudioEngine`) & Missing Audio Streaming
- **Subsystem:** Assets & Audio (`crates/assets`, `crates/audio`)
- **Evidence:** `assets/src/derived.rs:21, 38-49` (`register_identity` grows 0.6–1.4 MB/s during Character Studio drags, never pruned); `assets/src/hidden.rs:83-88, 129-134` (`hidden_models` uncapped); `assets/src/memory.rs:38-79` omits derived/hidden/identities; `audio/src/lib.rs:467` (`AudioEngine::decoded` caches all `StaticSoundData` forever with no streaming audio).
- **Severity:** P1 (Stability / Memory Leak)
- **User/Developer Impact:** Long Character Studio sessions leak hundreds of MB; music-heavy games hold decompressed PCM for every track in RAM forever.
- **Root Cause:** Prototype caches lacked eviction policies and streaming decode paths.
- **Recommended Solution:** Prune unreferenced `identities` & `hidden_models` on drag-end/LRU cap (matching `derived.models`'s `KEPT = 96`); include them in `Assets::memory()`; add Kira `StreamingSoundData` for long tracks (>10s or `Streamed = true`) and an LRU byte budget on `AudioEngine::decoded`.
- **Dependencies:** None.
- **Acceptance Criteria:** 1,000 simulated Character Studio slider drags plateau at bounded RAM; `Assets::memory()` accurately accounts for derived assets; streaming audio plays without loading full PCM.
- **Required Tests:** Memory-bound regression tests in `crates/assets` and streaming playback test in `crates/audio`.
- **Potential Regressions:** Ensure active character identities and looping SFX aren't evicted mid-use.
- **Estimated Complexity:** Medium (2–3 days).

### REM-P1-07 — `mesh_edit::csg` Degenerate O(N)-Depth Recursive BSP Stack-Overflow Risk
- **Subsystem:** Mesh Editing & CSG (`crates/mesh_edit`)
- **Evidence:** `crates/mesh_edit/src/csg.rs:441-535` splits always on `polygons.first()` with recursive `build`, `clip_polygons`, `clip_to`, `invert`, `all_polygons`, and recursive `Box<Node>` `Drop`.
- **Severity:** P1 (Crash / Stack Overflow)
- **User/Developer Impact:** Calling `UnionAsync`/`SubtractAsync` on high-segment spheres/cylinders can overflow the main/worker stack and crash the editor or player.
- **Root Cause:** Naive textbook BSP implementation without split heuristic or iterative traversal/drop.
- **Recommended Solution:** Select split planes via bounded candidate sampling (minimizing splits & imbalance), convert traversal and `Drop` to iterative stacks, and enforce a max-depth guard.
- **Dependencies:** None.
- **Acceptance Criteria:** CSG union/subtract on two 2,000-triangle spheres completes cleanly without stack overflow.
- **Required Tests:** Stress test in `crates/mesh_edit/src/csg_tests.rs` on high-tessellation convex spheres/cylinders.
- **Potential Regressions:** Verify all existing CSG tests produce watertight geometry.
- **Estimated Complexity:** Medium (1–2 days).

### REM-P1-08 — Unverified CI Packaging Downloads & `pkexec` SHA-256 Defense-in-Depth
- **Subsystem:** Release & Updater (`.github/workflows/release.yml`, `crates/updater`)
- **Evidence:** `release.yml:391-395` downloads `nfpm` and `appimagetool` (`continuous` tag) via `curl -sSL` with no SHA-256 verification; `updater/src/package.rs:39, 69-90` passes `sha256` to root `pkexec /bin/sh` without validating 64-char hex format; `mktemp -d` (`0700`) can trigger `_apt` warnings.
- **Severity:** P1 (Supply-Chain & Privilege-Boundary Hardening)
- **User/Developer Impact:** Compromised upstream `continuous` AppImage tag could backdoor Linux releases; caller misuse of `pub fn install_system_package` could inject into root shell.
- **Root Cause:** Quick CI setup without checksum pinning; missing input validation on public helper.
- **Recommended Solution:** Pin `nfpm`, `appimagetool`, `pillow`, and `dmgbuild` to exact versions with SHA-256 checks (like `fetch_luau_lsp.py`); validate `sha256.len() == 64 && sha256.bytes().all(|b| b.is_ascii_hexdigit())` inside `install_system_package`; `chmod 755 "$dir"` for `_apt`; re-verify SHA-256 in `replace_appimage`.
- **Dependencies:** None.
- **Acceptance Criteria:** `install_system_package` rejects non-hex `sha256`; `release.yml` verifies SHA-256 of every downloaded build tool.
- **Required Tests:** Unit tests in `crates/updater` for malformed `sha256` rejection.
- **Potential Regressions:** None.
- **Estimated Complexity:** Low (3–4 hours).

## P2 — Confirmed Moderate Defects

### REM-P2-01 — Six UI Widgets Silently Zero Percent (`%`) Margins and Padding
- **Subsystem:** UI (`crates/ui`)
- **Evidence:** `crates/ui/src/{chat.rs:77, dialogue_box.rs:36, hotbar.rs:85, interact.rs:1001, leaderboard.rs:167, prompt.rs:162}` match `taffy::LengthPercentage::Percent(_)` to `0.0`.
- **Severity:** P2
- **User/Developer Impact:** Responsive `%`-based CSS margins/padding silently fail on built-in game widgets.
- **Root Cause:** Local helper omitted parent container dimension lookup.
- **Recommended Solution:** Resolve `Percent(p)` against the parent's width/height in a shared helper used by all 6 widgets.
- **Dependencies:** None.
- **Acceptance Criteria:** `%` margins/padding render accurately across all 6 widgets at 720p, 1080p, and 1440p.
- **Required Tests:** Layout unit tests in `crates/ui/src/widget_tests.rs`.
- **Potential Regressions:** Check existing built-in stylesheets don't rely on ignored `%` values.
- **Estimated Complexity:** Low (3–4 hours).

### REM-P2-02 — `terrain::Job::try_take` Silently Swallows Worker Panics
- **Subsystem:** Terrain (`crates/terrain`)
- **Evidence:** `crates/terrain/src/job.rs:78-84` collapses worker panics, spawn failures, and cancellation into `Some(None)` via `.join().ok().flatten()`.
- **Severity:** P2
- **User/Developer Impact:** Background terrain generation/erosion panics disappear silently with no log or error banner.
- **Root Cause:** Over-simplified `Option<Option<T>>` return type.
- **Recommended Solution:** Return `JobPoll::Pending | Ready(T) | Cancelled | Failed(String)` and log worker panics explicitly.
- **Dependencies:** None.
- **Acceptance Criteria:** Panicking terrain job surfaces `Failed` and logs diagnostics.
- **Required Tests:** Unit test in `crates/terrain` verifying panic propagation.
- **Potential Regressions:** Update callers in `editor` / `terrain`.
- **Estimated Complexity:** Low (2 hours).

### REM-P2-03 — Documentation & Wiki Drift (58 `check-wiki.py` Warnings + Undocumented `unsafe`)
- **Subsystem:** Documentation (`wiki/`, `GEMINI.md`, `Cargo.toml`)
- **Evidence:** `python3 scripts/check-wiki.py` reports 58 warnings (2 broken `verified_commit` hashes, 56 stale pages); `System-Inventory.md` cites 33 crates / wgpu 25 / `0.2.0-alpha.1`; `GEMINI.md` and `Cargo.toml` omit `crates/release/src/console.rs` from `unsafe_code` exemptions.
- **Severity:** P2
- **User/Developer Impact:** Engineers and AI agents rely on stale crate counts, versions, and safety invariants.
- **Root Cause:** Rapid development without enforcing `check-wiki.py` on milestones.
- **Recommended Solution:** Update `GEMINI.md`, `Cargo.toml` comment, `System-Inventory.md`, and broken `verified_commit` hashes in `Networking.md` and `World-Streaming.md`.
- **Dependencies:** None.
- **Acceptance Criteria:** `python3 scripts/check-wiki.py` reports 0 errors and 0 invalid-commit warnings; `unsafe` exemptions match source exactly.
- **Required Tests:** `python3 scripts/check-wiki.py`.
- **Potential Regressions:** None.
- **Estimated Complexity:** Low (2–4 hours).

---

# Part II — Engineering Recommendations & Strategic Opportunities

## P1 — Foundational Production-Readiness Recommendations

### REC-P1-01 — Persist `wgpu::PipelineCache` & Slim Shadow Instance Stream (172B → 88B)
- **Subsystem:** Rendering (`crates/render`)
- **Evidence:** 30+ pipeline creation sites pass `cache: None`; `renderer/gpu.rs:647` feeds full 172-byte `InstanceRaw` to shadow passes that only read transform + skin/pose indices.
- **Severity:** P1 (Recommendation)
- **User/Developer Impact:** Slower cold startup shader compilation and ~2× shadow instance upload/fetch bandwidth.
- **Root Cause:** Pipeline caching not wired on `wgpu` 30 upgrade; shared vertex layout reused for shadows.
- **Recommended Solution:** Persist `wgpu::PipelineCache` per adapter/driver key in `.axcenity/shader-cache/`; introduce an 88-byte `ShadowInstanceRaw` stream (roadmap L5).
- **Dependencies:** None.
- **Acceptance Criteria:** Warm startup avoids full shader recompilation; shadow-pass GPU time measurably drops on `bench_10000_parts` with zero visual diff.
- **Required Tests:** Render visual regression tests + startup timing benchmark.
- **Potential Regressions:** Invalidate cache cleanly on shader hash or driver version change.
- **Estimated Complexity:** Medium (2–3 days).

### REC-P1-02 — Single-Serialize Multiplayer Broadcasts & Eliminate O(N) Peer Scans
- **Subsystem:** Multiplayer (`crates/net`)
- **Evidence:** `game.rs:1288-1293` re-serializes `Message::Update` to `postcard` N× per frame; `session.rs:426` linearly scans connections; `streaming.rs:247` calls `rebuild_parts` every 200ms per peer even without changes.
- **Severity:** P1 (Recommendation)
- **User/Developer Impact:** Server CPU scales poorly at 32–64+ concurrent players.
- **Root Cause:** Convenience per-peer send helper.
- **Recommended Solution:** Serialize shared `Update` payload once into `Bytes`/`Arc<[u8]>` and fan out; index connections in `HashMap<ClientId, usize>`; dirty-flag `Streams::for_peer`.
- **Dependencies:** REM-P1-04.
- **Acceptance Criteria:** 32-bot `axcenity-server` benchmark shows measurable frame-time reduction.
- **Required Tests:** Existing 114 `net` tests + `axcenity-bots` 32-client benchmark.
- **Potential Regressions:** Keep per-peer visibility filtering (streaming) separate from global updates.
- **Estimated Complexity:** Medium (1–2 days).

### REC-P1-03 — Mount `.axgame` Archives In-Place (Eliminate Temp-Dir Unpacking)
- **Subsystem:** Player & Project (`crates/player`, `crates/project`, `crates/assets`)
- **Evidence:** `player/src/main.rs:155-189` reads the entire `.axgame` into RAM and unpacks every file to a temp dir on every launch; `packed.rs:149-214` holds ~2× game size in RAM while packing.
- **Severity:** P1 (Recommendation)
- **User/Developer Impact:** Slow cold game launches, doubled disk usage, and leaked temp folders on crash/SIGKILL.
- **Root Cause:** `Assets` historically assumed loose filesystem paths.
- **Recommended Solution:** Add a `VfsSource::PackedArchive` backend in `crates/assets` that reads the `.axgame` index once and decompresses entries on demand via random-access `File` seek; stream-write entries in `pack_game_reporting`.
- **Dependencies:** REM-P1-02.
- **Acceptance Criteria:** `axcenity-player` launches `.axgame` with zero temp-dir extraction and <50% peak export RAM.
- **Required Tests:** Round-trip packed game launch test in `crates/player` and `crates/project`.
- **Potential Regressions:** Verify external tools or Luau `ContentProvider` paths resolve identically.
- **Estimated Complexity:** High (4–6 days).

### REC-P1-04 — Continue Splitting Monolithic `editor` Crate (`EditorApp` 171 Fields / 257k LOC)
- **Subsystem:** Editor (`crates/editor`)
- **Evidence:** `crates/editor` is 257,297 LOC (33% of entire workspace); `EditorApp` (`app.rs:208-604`) has 171 fields.
- **Severity:** P1 (Recommendation — Roadmap L6/G11)
- **User/Developer Impact:** 5–6s incremental rebuilds, stack-size risk in debug tests, and high merge-conflict rate.
- **Root Cause:** Rapid studio growth inside one binary crate.
- **Recommended Solution:** Extract `character_workspace` + `character_studio` into `crates/editor-character-studio`, and `animation` into `crates/editor-animation`, following the clean action-returning pattern of `editor-ui-designer`.
- **Dependencies:** REM-P0-02.
- **Acceptance Criteria:** `crates/editor` drops below 200k LOC; `EditorApp` field count reduced by >=30.
- **Required Tests:** Full `editor` + new sub-crate unit and snapshot tests.
- **Potential Regressions:** Preserve crate boundary invariants (`player` unaffected).
- **Estimated Complexity:** High (1–2 weeks, incremental PRs).

## P2 — Significant Functionality & Workflow Recommendations

### REC-P2-01 — Complete Default Realistic Character Assets, Draft Filter, Synced LODs & Eye Shader
- **Subsystem:** Characters & Rendering (`crates/avatar`, `crates/render`, `crates/editor`)
- **Evidence:** Default realistic character ships in underwear + 1 draft hair + 3 failing draft garments; Character Studio lacks "Show drafts" filter (`ItemManifest::is_draft`); per-part LOD switching causes skin poke-through; Eye shading model absent.
- **Severity:** P2 (Recommendation)
- **User/Developer Impact:** New projects spawn unfinished characters; visible visual defects in Character Studio.
- **Root Cause:** Character Studio overhaul (CO7/CO10/CO18) still in progress.
- **Recommended Solution:** Wire `is_draft` filter in Character Studio; implement synchronized character LOD sets (CO18); implement Eye shading model (cornea refraction/iris parallax); sign off starter hair + wardrobe.
- **Dependencies:** REM-P1-06.
- **Acceptance Criteria:** Starter cast passes `avatar` fit gates with 0 poke-through across LOD0–LOD2; Eye shader covered by headless render test.
- **Required Tests:** `avatar` fit gate tests + `render` eye-shading snapshot test.
- **Potential Regressions:** Monitor GPU frame cost on 20-character crowd benchmark.
- **Estimated Complexity:** High (2–3 weeks).

### REC-P2-02 — Spatial `ItemChunk`s for Retained Draw List & Decouple `TerrainCache`
- **Subsystem:** Scene Bridge (`crates/scene`)
- **Evidence:** `retained.rs:319, 338-384` clones all `DrawItem`s per frame and re-walks entire Workspace on any structural change; `TerrainCache` owns `draw_cache` and `streaming`.
- **Severity:** P2 (Recommendation)
- **User/Developer Impact:** Limits CPU scaling in 10k–50k part worlds.
- **Root Cause:** M4 retained cache stopped before chunking static geometry.
- **Recommended Solution:** Group static parts into spatial `ItemChunk`s, remove legacy non-retained draw path, and decouple `DrawCache` from `TerrainCache`.
- **Dependencies:** None.
- **Acceptance Criteria:** `bench_10000_parts` `build_draw_list` drops below 2 ms; zero visual diffs.
- **Required Tests:** `scene/src/retained_tests.rs` + `bench_10000_parts`.
- **Potential Regressions:** Verify visibility/transform dirty tracking on reparented models.
- **Estimated Complexity:** High (1 week).

### REC-P2-03 — Consolidate Ad-Hoc Background Threads & Add Dynamic Navmesh Obstacles
- **Subsystem:** Concurrency & Navigation (`crates/nav`, `crates/terrain`, `crates/procedural`, `crates/mesh_edit`)
- **Evidence:** `nav/shared.rs:49-53` spawns a fresh OS thread per rebuild and detaches on drop; `nav/builder.rs:12-15` ignores unanchored parts; `terrain`/`procedural`/`mesh_edit` spawn ad-hoc scoped threads.
- **Severity:** P2 (Recommendation)
- **User/Developer Impact:** Unbounded thread churn; AI agents walk through dynamic doors/crates.
- **Root Cause:** Per-crate threading helpers built independently.
- **Recommended Solution:** Introduce a shared bounded background job pool with cancellation tokens; add dynamic obstacle carving/avoidance for unanchored moving parts in `nav`.
- **Dependencies:** REM-P2-02.
- **Acceptance Criteria:** Zero detached threads on `SharedNavMesh::drop`; agents route around dynamic obstacles.
- **Required Tests:** `nav` dynamic obstacle tests + drop-while-building test.
- **Potential Regressions:** Ensure navmesh rebuild latency stays within 0.5s budget.
- **Estimated Complexity:** Medium-High (4–5 days).

### REC-P2-04 — Async Fracture Preloading & Complete CSS Layout (`flex-wrap`, `min/max`)
- **Subsystem:** Physics & UI (`crates/physics`, `crates/ui`)
- **Evidence:** `physics/src/fracture.rs:144-145` calls `assets.model_blocking` synchronously on sim thread; `ui` lacks `flex-wrap`, `min/max-width/height`, `display: grid`.
- **Severity:** P2 (Recommendation)
- **User/Developer Impact:** Frame hitch on first fracture; limited responsive UI layouts.
- **Root Cause:** Synchronous fallback in fracture; partial CSS mapping over `taffy` (which already supports `flex-wrap` and `grid`).
- **Recommended Solution:** Preload fracture piece models asynchronously via `LoadTicket`; expose `taffy`'s `flex-wrap` and `min/max` sizing in `crates/ui/src/style.rs`.
- **Dependencies:** REM-P2-01.
- **Acceptance Criteria:** Zero blocking disk I/O in `fracture::shatter`; `flex-wrap` and `min/max` layout tests pass.
- **Required Tests:** `physics/src/fracture_tests.rs` + `ui/src/tests.rs`.
- **Potential Regressions:** None.
- **Estimated Complexity:** Medium (2–3 days).

### REC-P2-05 — Dedicated Server Observability (`/health`, Structured Logs) & Multi-Machine Soak (MP7)
- **Subsystem:** Dedicated Server & Multiplayer (`crates/server`, `crates/net`)
- **Evidence:** No `/health` endpoint or JSON logs; multiplayer only tested on loopback/emulated routers (MP7 two-machine 16–32 client run pending).
- **Severity:** P2 (Recommendation)
- **User/Developer Impact:** Cannot operate `axcenity-server` reliably in production containers or verify real-world NAT/packet-loss stability.
- **Root Cause:** Deferred to `online-services.md` O2 and `multiplayer-readiness.md` MP7.
- **Recommended Solution:** Add optional `--health-port` HTTP probe + structured JSON log flag; execute and record 16–32 real-client multi-machine soak test.
- **Dependencies:** REM-P0-06, REM-P1-01, REM-P1-03, REC-P1-02.
- **Acceptance Criteria:** 1-hour 32-client soak test with zero crashes, bounded RSS, and <16.6ms p99 tick.
- **Required Tests:** Automated soak report recorded in `wiki/Performance/Benchmark-History.md`.
- **Potential Regressions:** None.
- **Estimated Complexity:** Medium (3–4 days).

### REC-P2-06 — Coverage-Guided Fuzzing (`cargo-fuzz`) for Network, Collab & Archive Decoders
- **Subsystem:** Security QA (`crates/net`, `crates/relay`, `crates/collab`, `crates/project`)
- **Evidence:** Zero `cargo-fuzz` targets exist across the workspace (only 20k seeded rounds in `collab/src/wire_fuzz_tests.rs`).
- **Severity:** P2 (Recommendation)
- **User/Developer Impact:** Untrusted wire/file parsers remain vulnerable to panics or OOM on edge-case inputs.
- **Root Cause:** Local-only verification policy omitted fuzz targets.
- **Recommended Solution:** Add `fuzz/` targets for `net::wire`, `relay::protocol`, `collab::wire`, `project::packed`, and `assets::model`.
- **Dependencies:** REM-P0-10, REM-P1-02.
- **Acceptance Criteria:** 10M+ fuzz executions per target with zero panics or OOMs.
- **Required Tests:** Fuzz corpus regression tests committed to unit suites.
- **Potential Regressions:** None.
- **Estimated Complexity:** Medium (2–3 days).

## P3 — Optimization, Polish & Longer-Term Capabilities

### REC-P3-01 — Render Graph Migration (L8) & Hardware Ray Tracing DX12 Stabilization
- **Subsystem:** Rendering (`crates/render`)
- **Evidence:** `renderer/scene.rs` is 3,703 lines of hand-ordered passes; `EXPERIMENTAL_RAY_QUERY` disabled by default due to RX 9070 XT DX12 flashing (`adapters.rs:111-113`).
- **Severity:** P3 (Strategic)
- **User/Developer Impact:** High maintenance cost for adding render passes; HW RT unusable by default on AMD DX12.
- **Root Cause:** Deferred by ADR-0009; upstream `wgpu` DX12 ray-query/barrier issue.
- **Recommended Solution:** Introduce lightweight pass-dependency render graph (L8) once pass count grows further; isolate and report/workaround DX12 RX 9070 XT barrier/flashing bug.
- **Dependencies:** REC-P1-01.
- **Acceptance Criteria:** Pass ordering declared as graph; HW RT passes visual regression on DX12 + Vulkan without flashing.
- **Required Tests:** Full 422+ `render` GPU test suite.
- **Potential Regressions:** Risk of barrier/synchronization regressions across 47 shaders.
- **Estimated Complexity:** Very High (3–4 weeks).

### REC-P3-02 — Cross-Vendor VRAM Detection & Per-Script Luau Memory Cap
- **Subsystem:** Benchmarking & Scripting (`crates/editor-benchmark`, `crates/script`)
- **Evidence:** `editor-benchmark/src/video_memory.rs:87-106` only detects Linux VRAM on `amdgpu`; `script` has no per-script memory limit (`Known-Limitations.md`).
- **Severity:** P3 (Polish)
- **User/Developer Impact:** Preset recommender lacks VRAM on NVIDIA/Intel Linux & macOS; runaway script table allocation can OOM host.
- **Root Cause:** Vendor-specific sysfs lookup; Luau allocator limit not wired.
- **Recommended Solution:** Query `wgpu` adapter/DXGI/NVML/sysfs fallbacks where safe; wire `mlua::Lua::set_memory_limit` per script context.
- **Dependencies:** None.
- **Acceptance Criteria:** Runaway `table` allocation triggers clean Luau OOM error instead of process abort.
- **Required Tests:** Script memory-limit unit test in `crates/script`.
- **Potential Regressions:** Tune default limit high enough for complex games (e.g., 256–512 MiB).
- **Estimated Complexity:** Medium (2–3 days).

### REC-P3-03 — `editor` Script Autocomplete & Non-Blocking MCP `game_run`
- **Subsystem:** Editor & MCP (`crates/editor`, `crates/mcp`)
- **Evidence:** Built-in script editor lacks full project-wide `luau-lsp` type-checking; MCP `game_run` (`editor/src/app/mcp/game_run.rs:79-90`) blocks editor main thread up to 60s.
- **Severity:** P3 (Workflow Polish)
- **User/Developer Impact:** Weak in-editor Luau DX compared to Roblox Studio; editor freezes during MCP game runs.
- **Root Cause:** Incremental editor feature buildout.
- **Recommended Solution:** Extend `luau-lsp` workspace indexing beyond open tabs; slice MCP `game_run` frames across editor `update()` ticks or run on a background `axcenity-test` worker.
- **Dependencies:** REM-P0-09.
- **Acceptance Criteria:** Editor remains responsive during 60s MCP `game_run`.
- **Required Tests:** MCP responsiveness test.
- **Potential Regressions:** None.
- **Estimated Complexity:** Medium (3–5 days).

### REC-P3-04 — Native Extension Seam & Optional Online Services (O1–O9)
- **Subsystem:** Extensibility & Online Services (`docs/research/online-services.md`)
- **Evidence:** No native plugin ABI (requires Rust fork); `crates/online` is 0% implemented.
- **Severity:** P3 (Strategic Opportunity)
- **User/Developer Impact:** Limits third-party native ecosystem and managed multiplayer hosting.
- **Root Cause:** Intentionally deferred until core engine stabilizes.
- **Recommended Solution:** Revisit only after Phases 0–4 complete; start with O1 (signed game delta manifests) and O2 (server containerization).
- **Dependencies:** Phases 0–4 complete.
- **Acceptance Criteria:** ADR approved before implementation.
- **Required Tests:** End-to-end delta update and container smoke tests.
- **Potential Regressions:** Keep `player`/`server` free of mandatory cloud dependencies.
- **Estimated Complexity:** Very High (multi-month).
