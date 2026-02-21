import { useEffect } from "react";
import { useStore } from "../store";
import {
  listSessions,
  listProjects,
  getConfig,
  onStatusChanged,
  onPrDetected,
} from "../lib/tauri";

/**
 * Hook that initializes session data and subscribes to backend events.
 * Should be called once at the app root level.
 */
export function useSessionInit() {
  const {
    setSessions,
    setProjects,
    setConfig,
    updateSessionStatus,
    updateSessionPrUrl,
  } = useStore();

  useEffect(() => {
    // Load initial data
    const init = async () => {
      try {
        const [sessions, projects, config] = await Promise.all([
          listSessions(),
          listProjects(),
          getConfig(),
        ]);
        setSessions(sessions);
        setProjects(projects);
        setConfig(config);
      } catch (err) {
        console.error("Failed to load initial data:", err);
      }
    };

    init();

    // Subscribe to status changes
    let unlistenStatus: (() => void) | null = null;
    let unlistenPr: (() => void) | null = null;

    const setupListeners = async () => {
      unlistenStatus = await onStatusChanged((event) => {
        updateSessionStatus(
          event.session_id,
          event.new_status,
          event.name
        );
      });

      unlistenPr = await onPrDetected((event) => {
        updateSessionPrUrl(event.session_id, event.url);
      });
    };

    setupListeners();

    return () => {
      unlistenStatus?.();
      unlistenPr?.();
    };
  }, []);
}
