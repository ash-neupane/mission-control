import { useEffect } from "react";
import { useStore } from "../store";
import { killSession as killSessionApi } from "../lib/tauri";

export default function KillConfirmDialog() {
  const { killConfirmSessionId, sessions, setKillConfirm, removeSession } =
    useStore();

  const session = sessions.find((s) => s.id === killConfirmSessionId);

  useEffect(() => {
    if (!killConfirmSessionId) return;

    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "y" || e.key === "Y" || e.key === "Enter") {
        e.preventDefault();
        e.stopPropagation();
        if (killConfirmSessionId) {
          killSessionApi(killConfirmSessionId)
            .then(() => {
              removeSession(killConfirmSessionId);
              setKillConfirm(null);
            })
            .catch(console.error);
        }
      } else if (e.key === "n" || e.key === "N" || e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        setKillConfirm(null);
      }
    };

    window.addEventListener("keydown", handleKey, true);
    return () => window.removeEventListener("keydown", handleKey, true);
  }, [killConfirmSessionId]);

  if (!killConfirmSessionId || !session) return null;

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="bg-cmux-surface border border-cmux-border rounded-lg w-[350px] p-4 shadow-2xl">
        <h2 className="text-[13px] font-semibold text-cmux-text-primary mb-3">
          Kill Session {session.number}?
        </h2>
        <p className="text-[11px] text-cmux-text-secondary mb-4">
          This will terminate{" "}
          <span className="text-cmux-text-primary font-semibold">
            {session.name}
          </span>{" "}
          ({session.project_name}). The PTY process will be killed.
        </p>
        <div className="flex gap-3 text-[10px] text-cmux-text-muted">
          <span>
            <kbd className="px-1 py-0.5 rounded bg-cmux-border text-cmux-text-secondary text-[9px]">
              y
            </kbd>{" "}
            confirm
          </span>
          <span>
            <kbd className="px-1 py-0.5 rounded bg-cmux-border text-cmux-text-secondary text-[9px]">
              n
            </kbd>{" "}
            cancel
          </span>
        </div>
      </div>
    </div>
  );
}
