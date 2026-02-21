import type { Session } from "../types";
import { statusColors } from "../lib/colors";
import { useStore } from "../store";

interface SessionPillProps {
  session: Session;
  isActive?: boolean;
}

export default function SessionPill({
  session,
  isActive = false,
}: SessionPillProps) {
  const focusSession = useStore((s) => s.focusSession);
  const color = statusColors[session.status];

  return (
    <button
      onClick={() => focusSession(session.id)}
      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-mono transition-colors ${
        isActive
          ? "bg-cmux-border text-cmux-text-primary"
          : "bg-cmux-surface text-cmux-text-secondary hover:text-cmux-text-primary"
      }`}
      title={`${session.number}: ${session.name}`}
    >
      <span
        className="w-1.5 h-1.5 rounded-full"
        style={{ backgroundColor: color }}
      />
      <span>{session.number}</span>
    </button>
  );
}
