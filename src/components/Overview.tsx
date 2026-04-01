import { useStore } from "../store";
import SessionCell from "./SessionCell";

export default function Overview() {
  const sessions = useStore((s) => s.sessions);
  const selectedOverviewIndex = useStore((s) => s.selectedOverviewIndex);

  const gridCols = getGridCols(sessions.length);

  if (sessions.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center max-w-[280px]">
          <p className="text-cmux-text-primary text-sm font-semibold mb-1">
            c-mux
          </p>
          <p className="text-cmux-text-secondary text-[11px] mb-4">
            Launch AI coding agents in parallel and supervise them from one screen.
          </p>
          <kbd
            className="inline-block px-3 py-1.5 rounded bg-cmux-surface border border-cmux-border text-cmux-text-primary text-[12px] font-bold cursor-pointer hover:border-cmux-working transition-colors"
            onClick={() => useStore.getState().toggleNewSessionModal()}
          >
            n
          </kbd>
          <p className="text-cmux-text-muted text-[10px] mt-2">
            press to start your first session
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
        {sessions.map((session, index) => (
          <SessionCell
            key={session.id}
            session={session}
            selected={index === selectedOverviewIndex}
          />
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
