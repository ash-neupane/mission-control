import { useState, useEffect } from "react";
import type { Session } from "../types";
import { statusColors, statusLabels } from "../lib/colors";
import { useStore } from "../store";
import { openUrl } from "../lib/tauri";

interface SidePanelProps {
  session: Session;
}

function useElapsedTime(startedAt: number): string {
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  useEffect(() => {
    const id = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000);
    return () => clearInterval(id);
  }, []);
  const elapsed = Math.max(0, now - startedAt);
  const minutes = Math.floor(elapsed / 60);
  const seconds = elapsed % 60;
  return `${minutes}m ${seconds}s`;
}

export default function SidePanel({ session }: SidePanelProps) {
  const sessions = useStore((s) => s.sessions);
  const otherSessions = sessions.filter((s) => s.id !== session.id);

  const timeStr = useElapsedTime(session.started_at);

  const statusColor = statusColors[session.status];
  const statusLabel = statusLabels[session.status];

  return (
    <div className="w-[220px] flex-shrink-0 bg-cmux-surface border-l border-cmux-border overflow-y-auto">
      <div className="p-3 space-y-4">
        {/* Session Info */}
        <section>
          <h3 className="text-[9px] font-bold uppercase text-cmux-text-muted mb-2 tracking-wider">
            Session Info
          </h3>
          <div className="space-y-1.5 text-[11px]">
            <InfoRow label="Project" value={session.project_name} />
            {session.branch && (
              <InfoRow
                label="Branch"
                value={session.branch}
                truncate
              />
            )}
            <InfoRow label="Time" value={timeStr} />
            <InfoRow label="Agent" value={session.agent} />
            <div className="flex items-center justify-between">
              <span className="text-cmux-text-muted">Status</span>
              <span
                className="font-bold text-[9px] uppercase"
                style={{ color: statusColor }}
              >
                {statusLabel}
              </span>
            </div>
          </div>
        </section>

        {/* PR URL */}
        {session.pr_url && (
          <section>
            <h3 className="text-[9px] font-bold uppercase text-cmux-text-muted mb-2 tracking-wider">
              Pull Request
            </h3>
            <a
              className="text-[10px] text-cmux-working hover:underline break-all"
              href="#"
              onClick={(e) => {
                e.preventDefault();
                openUrl(session.pr_url!).catch(console.error);
              }}
            >
              {session.pr_url}
            </a>
          </section>
        )}

        {/* Other Sessions */}
        {otherSessions.length > 0 && (
          <section>
            <h3 className="text-[9px] font-bold uppercase text-cmux-text-muted mb-2 tracking-wider">
              Other Sessions
            </h3>
            <div className="space-y-1">
              {otherSessions.map((s) => (
                <OtherSessionRow key={s.id} session={s} />
              ))}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

function InfoRow({
  label,
  value,
  truncate = false,
}: {
  label: string;
  value: string;
  truncate?: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="text-cmux-text-muted flex-shrink-0">{label}</span>
      <span
        className={`text-cmux-text-secondary ${truncate ? "truncate" : ""}`}
        title={truncate ? value : undefined}
      >
        {value}
      </span>
    </div>
  );
}

function OtherSessionRow({ session }: { session: Session }) {
  const focusSession = useStore((s) => s.focusSession);
  const color = statusColors[session.status];

  return (
    <button
      onClick={() => focusSession(session.id)}
      className="flex items-center gap-2 w-full text-left px-1.5 py-1 rounded hover:bg-cmux-border transition-colors text-[11px]"
    >
      <span className="text-cmux-text-muted font-bold w-3 text-right">
        {session.number}
      </span>
      <span
        className="w-1.5 h-1.5 rounded-full flex-shrink-0"
        style={{ backgroundColor: color }}
      />
      <span className="text-cmux-text-secondary truncate">
        {session.name}
      </span>
    </button>
  );
}
