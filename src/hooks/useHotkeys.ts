import { useEffect, useRef } from "react";
import { useStore } from "../store";
import { openUrl } from "../lib/tauri";

/**
 * Global hotkey handler for c-mux.
 *
 * Design principles for focus mode ergonomics:
 * - Bare keys (letters, numbers, Tab, Escape) MUST pass through to the terminal.
 * - All c-mux shortcuts in focus mode use modifier keys (Ctrl, Alt).
 * - Overview mode is safe to use bare keys since there's no active terminal.
 */
export function useHotkeys() {
  // Track last Escape time for double-Escape detection in focus mode
  const lastEscapeRef = useRef<number>(0);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const {
        viewMode,
        focusedSessionId,
        showNewSessionModal,
        showHelpOverlay,
        killConfirmSessionId,
        sessions,
      } = useStore.getState();

      // If modal is open, don't handle global hotkeys
      if (showNewSessionModal) return;

      // Kill confirm dialog is handled by its own listener
      if (killConfirmSessionId) return;

      // Help overlay
      if (showHelpOverlay) {
        if (e.key === "Escape" || e.key === "?") {
          e.preventDefault();
          useStore.getState().toggleHelpOverlay();
        }
        return;
      }

      // Global: Ctrl+N — new session
      if (e.ctrlKey && e.key === "n") {
        e.preventDefault();
        useStore.getState().toggleNewSessionModal();
        return;
      }

      // Overview mode hotkeys — bare keys are safe here (no active terminal)
      if (viewMode === "overview") {
        // Number keys 1-9: focus session
        if (e.key >= "1" && e.key <= "9" && !e.ctrlKey && !e.altKey) {
          const num = parseInt(e.key, 10);
          const session = sessions.find((s) => s.number === num);
          if (session) {
            e.preventDefault();
            useStore.getState().focusSession(session.id);
          }
          return;
        }

        // Arrow keys: navigate selected session in overview
        if (e.key === "ArrowDown" || e.key === "ArrowUp" ||
            e.key === "ArrowLeft" || e.key === "ArrowRight") {
          e.preventDefault();
          const { moveOverviewSelection } = useStore.getState();
          const cols = sessions.length <= 1 ? 1 : sessions.length <= 4 ? 2 : 3;
          let delta = 0;
          if (e.key === "ArrowDown") delta = cols;
          else if (e.key === "ArrowUp") delta = -cols;
          else if (e.key === "ArrowRight") delta = 1;
          else if (e.key === "ArrowLeft") delta = -1;
          moveOverviewSelection(delta, sessions.length);
          return;
        }

        // Enter: focus selected session
        if (e.key === "Enter" && sessions.length > 0) {
          e.preventDefault();
          const { selectedOverviewIndex } = useStore.getState();
          const idx = Math.min(selectedOverviewIndex, sessions.length - 1);
          useStore.getState().focusSession(sessions[idx].id);
          return;
        }

        // Tab: jump to next NeedsInput session
        if (e.key === "Tab") {
          e.preventDefault();
          const next = useStore
            .getState()
            .getNextNeedsInputSession(focusedSessionId ?? undefined);
          if (next) {
            useStore.getState().focusSession(next.id);
          }
          return;
        }

        // n: new session
        if (e.key === "n" && !e.ctrlKey) {
          e.preventDefault();
          useStore.getState().toggleNewSessionModal();
          return;
        }

        // q: kill selected session
        if (e.key === "q" && !e.ctrlKey) {
          e.preventDefault();
          if (sessions.length > 0) {
            const { selectedOverviewIndex } = useStore.getState();
            const idx = Math.min(selectedOverviewIndex, sessions.length - 1);
            useStore.getState().setKillConfirm(sessions[idx].id);
          }
          return;
        }

        // ?: help
        if (e.key === "?") {
          e.preventDefault();
          useStore.getState().toggleHelpOverlay();
          return;
        }

        return;
      }

      // Focus mode hotkeys — only modifier-based shortcuts to avoid stealing terminal input
      if (viewMode === "focus") {
        // Double-Escape: return to overview (two Escape presses within 400ms)
        if (e.key === "Escape") {
          const now = Date.now();
          if (now - lastEscapeRef.current < 400) {
            e.preventDefault();
            e.stopPropagation();
            lastEscapeRef.current = 0;
            useStore.getState().returnToOverview();
          } else {
            lastEscapeRef.current = now;
          }
          // Single Escape passes through to the terminal
          return;
        }

        // Alt+1-9: switch session (stays in focus)
        if (e.altKey && e.key >= "1" && e.key <= "9") {
          const num = parseInt(e.key, 10);
          const session = sessions.find((s) => s.number === num);
          if (session) {
            e.preventDefault();
            e.stopPropagation();
            useStore.getState().focusSession(session.id);
          }
          return;
        }

        // Alt+Tab: jump to next NeedsInput session
        if (e.altKey && e.key === "Tab") {
          e.preventDefault();
          e.stopPropagation();
          const next = useStore
            .getState()
            .getNextNeedsInputSession(focusedSessionId ?? undefined);
          if (next) {
            useStore.getState().focusSession(next.id);
          }
          return;
        }

        // Ctrl+Q: kill focused session
        if (e.ctrlKey && e.key === "q") {
          e.preventDefault();
          e.stopPropagation();
          if (focusedSessionId) {
            useStore.getState().setKillConfirm(focusedSessionId);
          }
          return;
        }

        // Ctrl+P: open PR URL
        if (e.ctrlKey && e.key === "p") {
          e.preventDefault();
          e.stopPropagation();
          const focused = sessions.find(
            (s) => s.id === focusedSessionId
          );
          if (focused?.pr_url) {
            openUrl(focused.pr_url).catch(console.error);
          }
          return;
        }

        // Ctrl+B: toggle side panel
        if (e.ctrlKey && e.key === "b") {
          e.preventDefault();
          e.stopPropagation();
          useStore.getState().toggleSidePanel();
          return;
        }

        // All other keys: pass through to terminal (don't prevent default)
      }
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, []);
}
