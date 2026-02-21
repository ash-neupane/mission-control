# c-mux Refactoring Plan

## Principles
- Fix real bugs and inconsistencies, don't add abstraction for its own sake
- Keep changes mechanical and verifiable via existing tests
- No new features — pure cleanup

---

## 1. Fix Lock Handling Consistency (High Priority — Bug Risk)

**Problem:** `commands.rs` mixes `.unwrap()` and `.map_err()` for mutex locks. If any lock poisons, the `.unwrap()` calls will panic and crash the app.

**Fix:** Replace all `.unwrap()` on mutex locks with `.map_err()` returning a user-facing error string. Create a small helper to reduce boilerplate:

```rust
fn lock_or_err<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, String> {
    mutex.lock().map_err(|e| format!("Internal lock error: {}", e))
}
```

**Files:** `commands.rs`

---

## 2. Extract Magic Numbers into Constants (Medium Priority)

**Problem:** Status detection thresholds are scattered as raw literals across `status.rs`. If tuning is needed, you have to hunt for every number.

**Fix:** Define constants at the top of `status.rs`:

```rust
const BUFFER_MAX_SIZE: usize = 2000;
const AUTO_NAME_THRESHOLD: usize = 2000;
const SIGNIFICANT_OUTPUT_BYTES: usize = 500;
const MIN_OUTPUT_BYTES: usize = 100;
const STUCK_TIMEOUT_SECS: u64 = 180;
const PTY_BATCH_INTERVAL_MS: u64 = 16;
```

**Files:** `status.rs`, `pty.rs`

---

## 3. Cache Compiled Regex (Medium Priority — Performance)

**Problem:** `strip_ansi()` in `status.rs` recompiles a regex on every call. This function runs on every PTY output chunk — hot path.

**Fix:** Use `std::sync::LazyLock` (stable since Rust 1.80) to compile the regex once.

**Files:** `status.rs`

---

## 4. Deduplicate Config Path Resolution (Low Priority)

**Problem:** `ProjectRegistry::config_path()` and `Config::config_path()` in `projects.rs` share identical logic for resolving `~/.cmux/`.

**Fix:** Extract a shared `cmux_dir()` function, have both call it and just append their filename.

**Files:** `projects.rs`

---

## 5. Fix Branch Prefix Hardcoding in Naming (Medium Priority — Bug)

**Problem:** `naming.rs` hardcodes `"cmux/session-"` as a prefix to strip, but the actual prefix comes from `Config.branch_prefix`. If the user changes the prefix in config, auto-naming breaks.

**Fix:** `name_from_branch()` should accept the configured prefix as a parameter.

**Files:** `naming.rs`, `commands.rs`

---

## 6. Fix Incomplete `needs_attention_since` in Rust Backend (Medium Priority — Bug)

**Problem:** `status.rs` line 157-159 has a TODO comment but no code — the `needs_attention_since` timestamp is never set on the backend. Frontend fills it in as a workaround, which means the Rust `Session` struct always has `None` for this field.

**Fix:** Set the timestamp in `commands.rs` when processing status change events, or in the session manager when status updates occur. Remove the frontend workaround so there's one source of truth.

**Files:** `status.rs`, `commands.rs`, `store.ts`

---

## 7. Fix Async Cleanup in useSession Hook (Medium Priority — Bug)

**Problem:** `useSession.ts` calls `setupListeners()` without awaiting. The cleanup function captures `unlistenStatus` and `unlistenPr` while they're still `null`. If the component unmounts before the promises resolve, listeners leak.

**Fix:** Use a `cancelled` flag pattern (like `usePty.ts` already does correctly).

**Files:** `hooks/useSession.ts`

---

## 8. Remove Dead `_last_unique_output` Field (Low Priority)

**Problem:** `StatusDetector` has `_last_unique_output` that's initialized but never read or written after construction.

**Fix:** Remove the field entirely.

**Files:** `status.rs`

---

## 9. Fix Overview Grid Cols (Low Priority — Bug)

**Problem:** `getGridCols()` in `Overview.tsx` has `count <= 2` and `count <= 4` both returning `"grid-cols-2"` — the `count <= 2` case is unreachable for 3-4 because it's checked first, but the logic reads confusingly and the spec says "2 = two columns, 3-4 = 2×2" which is the same grid-cols anyway. However, 1 session should be full width which is correct. The real issue is the duplicate branch is confusing.

**Fix:** Simplify and comment the grid logic to match the spec exactly.

**Files:** `components/Overview.tsx`

---

## Order of Implementation

1. Lock handling (highest risk fix)
2. `needs_attention_since` backend fix
3. Async cleanup fix in useSession
4. Regex caching
5. Magic number constants
6. Config path dedup
7. Branch prefix fix in naming
8. Remove dead field
9. Grid cols cleanup
