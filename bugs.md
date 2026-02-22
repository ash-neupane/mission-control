# c-mux Bug Report — QA Pass

Generated 2026-02-22. Tooling: `cargo test`, `cargo clippy --pedantic`, `tsc --noEmit`, manual code review.

---

## CRITICAL — Runtime crash / data loss

### BUG-01: Zombie child processes (pty.rs:68-73)

The `Child` handle returned by `spawn_command()` is extracted for its PID then immediately
dropped. On Linux this leaks a zombie process for every session, since no `waitpid()` is
ever issued. Over time this exhausts the PID table.

```rust
let child = pair.slave.spawn_command(cmd)...;
let child_pid = child.process_id().unwrap_or(0);
// `child` is dropped here — zombie remains until parent exits
```

**Fix:** Store the `Child` handle and reap it in `kill()` / when the reader thread detects EOF.

### BUG-02: kill() blocks Tauri command thread for 500ms while holding PtyPool lock (pty.rs:215-216)

`thread::sleep(Duration::from_millis(500))` is called inside `kill()` while the PtyPool
mutex is held. All concurrent PTY operations (`write_to_pty`, `resize_pty`, `create_session`)
are blocked for the entire 500ms grace period.

```rust
let _ = nix::sys::signal::kill(..., SIGHUP);
thread::sleep(Duration::from_millis(500));  // <— blocks everything
let _ = nix::sys::signal::kill(..., SIGKILL);
```

**Fix:** Release the lock before sleeping, or move the SIGHUP→wait→SIGKILL sequence to a
background thread and return immediately.

---

## HIGH — Broken feature / incorrect behavior

### BUG-03: OS notifications never fire (notifications.rs is dead code)

`should_notify()` and `notification_body()` are defined but never called anywhere. The
`StatusDetector::transition()` method emits a Tauri event for the frontend but never
triggers an OS notification. The notification plugin is loaded (`tauri_plugin_notification`)
but unused from Rust.

**Fix:** Call `should_notify()` inside `transition()` and emit a notification payload that
the frontend can forward to the notification API.

### BUG-04: Status detection immediately overrides PrReady (status.rs:100-108)

When a PR URL is detected, `transition(PrReady)` is called on line 100. But then
`detect_status()` runs unconditionally on lines 105-108 and can immediately override
PrReady → Working or NeedsInput in the same `feed()` call.

```rust
self.transition(SessionStatus::PrReady, app_handle);  // line 100
// ...
let new_status = self.detect_status(&clean_text);      // line 105
if new_status != self.current_status {
    self.transition(new_status, app_handle);            // line 107  ← overrides PrReady
}
```

**Fix:** Return early after transitioning to PrReady, or have `detect_status()` preserve
PrReady when a PR URL is known.

### BUG-05: Terminal PTY connection depends on external re-render (Terminal.tsx:120)

`usePty(sessionId, termRef.current, active)` is called during render with `termRef.current`.
On first render this is `null`. The useEffect in line 26 sets `termRef.current = term`, but
refs don't trigger re-renders. The `usePty` hook's internal effect has `[sessionId, terminal]`
in its dependency array — but `terminal` remains `null` until something else triggers a
re-render.

In practice this is masked by frequent store updates, but there is a race window where early
PTY output is lost.

**Fix:** Use `useState` for the terminal instance (or a `useCallback` ref) so the PTY hook
re-subscribes when the terminal is created.

---

## MEDIUM — UX / state consistency

### BUG-06: NewSessionModal setProjects uses stale closure (NewSessionModal.tsx:187)

`handleAddProject` calls `setProjects([project, ...projects])` where `projects` is the
*local component state* captured at render time, not the current store state. If the store
was updated between renders, the store gets overwritten with stale data.

```typescript
setProjects([project, ...projects]);  // projects is from useState, not store
```

**Fix:** Use `useStore.getState().projects` or call `setProjects` via a functional updater.

### BUG-07: KillConfirmDialog silently swallows kill failures (KillConfirmDialog.tsx:24)

`killSessionApi(...).catch(console.error)` logs to console but shows no UI feedback. If the
kill fails (process already exited, lock poisoned), the dialog stays open with no indication
of failure and the user has no recourse.

**Fix:** Add an error state to the dialog and display it.

### BUG-08: SidePanel elapsed time is never updated (SidePanel.tsx:14-17)

`Date.now() / 1000 - session.started_at` is computed once at render time. There is no
`setInterval` or re-render trigger, so the displayed time is always the value from the last
render and never ticks forward.

```typescript
const elapsed = Math.floor(Date.now() / 1000 - session.started_at);
```

**Fix:** Use a `useEffect` with `setInterval` to force re-render every second.

### BUG-09: Dead sessions remain in PtyPool after PTY reader exits (pty.rs reader thread)

When the reader thread exits (EOF or error), the `masters`, `child_pids`, and
`status_detectors` entries for that session remain in the PtyPool. There is no callback
to the SessionManager. The session appears alive in the UI but is actually dead.

**Fix:** Emit a "session-ended" event from the reader thread so the frontend can update,
or mark the session status as Done/Error.

---

## LOW — Code quality (not fixing in this pass)

| ID | File | Issue |
|----|------|-------|
| L-01 | `projects.rs:13` | `fs::create_dir_all().ok()` silently ignores dir creation failure |
| L-02 | `commands.rs:19-28` | `create_session` has 8 parameters (clippy threshold: 7) |
| L-03 | `session.rs:86` | `(i + 1) as u8` — safe for 1-9 but no bounds check |
| L-04 | `pty.rs:211,220` | `pid as i32` cast wraps on u32 > i32::MAX |
| L-05 | `status.rs:117-125` | Inefficient double-reverse to get last 5 lines |
| L-06 | Multiple files | `format!("...: {}", e)` instead of `format!("...: {e}")` (pedantic) |
