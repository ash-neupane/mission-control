import { useEffect } from "react";
import { useStore } from "../store";
import { openUrl } from "../lib/tauri";

/**
 * Global hotkey handler for c-mux.
 * Registers keyboard shortcuts that work across both Overview and Focus modes.
 */
export function useHotkeys() {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const {
        viewMode,
        focusedSessionId,
        showNewSessionModal,
        showHelpOverlay,
        sessions,
      } = useStore.getState();

      // If modal is open, don't handle global hotkeys
      if (showNewSessionModal) return;

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

      // Overview mode hotkeys
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

        // q: kill session (prompts confirmation for first visible session)
        if (e.key === "q" && !e.ctrlKey) {
          e.preventDefault();
          if (sessions.length > 0) {
            const { killConfirmSessionId } = useStore.getState();
            if (killConfirmSessionId) {
              useStore.getState().setKillConfirm(null);
            } else {
              useStore.getState().setKillConfirm(sessions[0].id);
            }
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

      // Focus mode hotkeys
      if (viewMode === "focus") {
        // Escape: return to overview
        if (e.key === "Escape") {
          e.preventDefault();
          e.stopPropagation();
          useStore.getState().returnToOverview();
          return;
        }

        // Number keys 1-9: switch session (stays in focus)
        if (e.key >= "1" && e.key <= "9" && !e.ctrlKey && !e.altKey) {
          const num = parseInt(e.key, 10);
          const session = sessions.find((s) => s.number === num);
          if (session && session.id !== focusedSessionId) {
            e.preventDefault();
            e.stopPropagation();
            useStore.getState().focusSession(session.id);
          }
          // Don't prevent default if it's the current session — let it pass through
          return;
        }

        // Tab: jump to next NeedsInput session
        if (e.key === "Tab") {
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
