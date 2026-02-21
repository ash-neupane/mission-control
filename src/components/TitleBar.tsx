import { useStore } from "../store";
import SessionPill from "./SessionPill";
import type { Session } from "../types";

export default function TitleBar() {
  const {
    viewMode,
    sessions,
    focusedSessionId,
  } = useStore();

  const needsInputCount = sessions.filter(
    (s) => s.status === "NeedsInput"
  ).length;
  const prReadyCount = sessions.filter(
    (s) => s.status === "PrReady"
  ).length;

  const focusedSession = sessions.find(
    (s) => s.id === focusedSessionId
  );

  if (viewMode === "focus" && focusedSession) {
    return (
      <div className="flex items-center justify-between h-8 px-3 bg-cmux-surface border-b border-cmux-border select-none">
        <div className="flex items-center gap-2 text-[12px]">
          <span className="text-cmux-text-muted font-bold">
            {focusedSession.number}
          </span>
          <span className="text-cmux-text-primary font-semibold">
            {focusedSession.name}
          </span>
          <span className="text-cmux-text-muted">│</span>
          <span className="text-cmux-text-secondary">
            {focusedSession.project_name}
          </span>
          {focusedSession.branch && (
            <>
              <span className="text-cmux-text-muted">│</span>
              <span className="text-cmux-text-secondary">
                {focusedSession.branch}
              </span>
            </>
          )}
        </div>
        <div className="flex items-center gap-1">
          {sessions
            .filter((s) => s.id !== focusedSessionId)
            .map((s) => (
              <SessionPill key={s.id} session={s} />
            ))}
        </div>
      </div>
    );
  }

  // Overview title bar
  return (
    <div className="flex items-center justify-between h-8 px-3 bg-cmux-surface border-b border-cmux-border select-none">
      <span className="text-[12px] font-bold text-cmux-text-primary">
        c-mux
      </span>
      <div className="flex items-center gap-3 text-[11px]">
        <span className="text-cmux-text-secondary">
          {sessions.length} session{sessions.length !== 1 ? "s" : ""}
        </span>
        {needsInputCount > 0 && (
          <span className="text-cmux-needs-input">
            {needsInputCount} need{needsInputCount !== 1 ? "" : "s"} input
          </span>
        )}
        {prReadyCount > 0 && (
          <span className="text-cmux-pr-ready">
            {prReadyCount} PR ready
          </span>
        )}
      </div>
    </div>
  );
}
