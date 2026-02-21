import { useStore } from "../store";
import Terminal from "./Terminal";
import SidePanel from "./SidePanel";

export default function FocusMode() {
  const { focusedSessionId, sessions, sidePanelVisible } = useStore();

  const session = sessions.find((s) => s.id === focusedSessionId);

  if (!session) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <p className="text-cmux-text-muted text-sm">Session not found</p>
      </div>
    );
  }

  return (
    <div className="flex-1 flex overflow-hidden">
      {/* Main terminal area */}
      <div className="flex-1 overflow-hidden">
        <Terminal
          sessionId={session.id}
          active={true}
          fontSize={13}
          className="h-full"
        />
      </div>

      {/* Side panel */}
      {sidePanelVisible && <SidePanel session={session} />}
    </div>
  );
}
