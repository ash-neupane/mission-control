import type { Session } from "../types";
import { statusColors, statusLabels, statusBorderClass } from "../lib/colors";
import Terminal from "./Terminal";
import { useStore } from "../store";

interface SessionCellProps {
  session: Session;
}

export default function SessionCell({ session }: SessionCellProps) {
  const focusSession = useStore((s) => s.focusSession);
  const color = statusColors[session.status];
  const label = statusLabels[session.status];

  const elapsed = Math.floor(Date.now() / 1000 - session.started_at);
  const minutes = Math.floor(elapsed / 60);
  const timeStr = minutes > 0 ? `${minutes}m` : "<1m";

  const tokensStr = session.tokens_used
    ? `${(session.tokens_used / 1000).toFixed(1)}k tokens`
    : "";

  return (
    <div
      className={`bg-cmux-surface rounded-lg overflow-hidden cursor-pointer hover:bg-opacity-80 transition-all ${statusBorderClass(session.status)}`}
      onClick={() => focusSession(session.id)}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-cmux-border">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-cmux-text-muted text-[11px] font-bold flex-shrink-0">
            {session.number}
          </span>
          <span className="text-[13px] font-semibold text-cmux-text-primary truncate">
            {session.name}
          </span>
        </div>
        <div className="flex items-center gap-2 flex-shrink-0">
          <span className="text-[11px] text-cmux-text-secondary">
            {session.project_name}
          </span>
          <span
            className="text-[9px] font-bold uppercase px-1.5 py-0.5 rounded"
            style={{
              color,
              backgroundColor: `${color}20`,
            }}
          >
            {label}
          </span>
        </div>
      </div>

      {/* Terminal Preview */}
      <div className="h-[150px] px-1">
        <Terminal
          sessionId={session.id}
          active={false}
          fontSize={9}
        />
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between px-3 py-1 border-t border-cmux-border text-[10px] text-cmux-text-muted">
        <span>{timeStr}</span>
        {tokensStr && <span>{tokensStr}</span>}
      </div>
    </div>
  );
}
