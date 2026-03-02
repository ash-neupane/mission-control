import { useStore } from "../store";
import SessionCell from "./SessionCell";

export default function Overview() {
  const sessions = useStore((s) => s.sessions);

  const gridCols = getGridCols(sessions.length);

  if (sessions.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center">
          <p className="text-cmux-text-secondary text-sm mb-2">
            No active sessions
          </p>
          <p className="text-cmux-text-muted text-xs">
            Press <kbd className="px-1 py-0.5 rounded bg-cmux-border text-cmux-text-secondary text-[10px] font-bold">n</kbd> to create a new session
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-auto p-3">
      <div
        className={`grid gap-3 h-full ${gridCols}`}
        style={{ gridAutoRows: "1fr" }}
      >
        {sessions.map((session) => (
          <SessionCell key={session.id} session={session} />
        ))}
      </div>
    </div>
  );
}

// Spec: 1→full, 2→2col, 3-4→2×2, 5-6→3×2, 7-9→3×3
function getGridCols(count: number): string {
  if (count <= 1) return "grid-cols-1";
  if (count <= 4) return "grid-cols-2";
  return "grid-cols-3";
}
