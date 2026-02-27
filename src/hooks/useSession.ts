import { useEffect } from "react";
import { useStore } from "../store";
import {
  listSessions,
  listProjects,
  getConfig,
  onStatusChanged,
  onPrDetected,
  onSessionNotification,
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
    let cancelled = false;
    let unlistenStatus: (() => void) | null = null;
    let unlistenPr: (() => void) | null = null;
    let unlistenNotify: (() => void) | null = null;

    const init = async () => {
      try {
        const [sessions, projects, config] = await Promise.all([
          listSessions(),
          listProjects(),
          getConfig(),
        ]);
        if (cancelled) return;
        setSessions(sessions);
        setProjects(projects);
        setConfig(config);
      } catch (err) {
        console.error("Failed to load initial data:", err);
      }

      // Subscribe to events after initial load
      if (cancelled) return;

      try {
        unlistenStatus = await onStatusChanged((event) => {
          if (cancelled) return;
          updateSessionStatus(
            event.session_id,
            event.new_status,
            event.name,
            event.needs_attention_since
          );
        });

        if (cancelled) {
          unlistenStatus();
          unlistenStatus = null;
          return;
        }

        unlistenPr = await onPrDetected((event) => {
          if (cancelled) return;
          updateSessionPrUrl(event.session_id, event.url);
        });

        if (cancelled) {
          unlistenPr();
          unlistenPr = null;
          return;
        }

        unlistenNotify = await onSessionNotification((event) => {
          if (cancelled) return;
          if (Notification.permission === "granted") {
            new Notification(event.title, { body: event.body });
          } else if (Notification.permission !== "denied") {
            Notification.requestPermission().then((perm) => {
              if (perm === "granted") {
                new Notification(event.title, { body: event.body });
              }
            });
          }
        });

        if (cancelled) {
          unlistenNotify();
          unlistenNotify = null;
        }
      } catch (err) {
        console.error("Failed to subscribe to session events:", err);
      }
    };

    init();

    return () => {
      cancelled = true;
      unlistenStatus?.();
      unlistenPr?.();
      unlistenNotify?.();
    };
  }, []);
}
