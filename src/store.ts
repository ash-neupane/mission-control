import { create } from "zustand";
import type {
  Session,
  ViewMode,
  SessionStatus,
  RegisteredProject,
  Config,
} from "./types";

interface AppState {
  // View state
  viewMode: ViewMode;
  focusedSessionId: string | null;
  showNewSessionModal: boolean;
  showHelpOverlay: boolean;
  sidePanelVisible: boolean;
  killConfirmSessionId: string | null;

  // Data
  sessions: Session[];
  projects: RegisteredProject[];
  config: Config | null;

  // Actions
  setViewMode: (mode: ViewMode) => void;
  focusSession: (sessionId: string) => void;
  returnToOverview: () => void;
  toggleNewSessionModal: () => void;
  closeNewSessionModal: () => void;
  toggleHelpOverlay: () => void;
  toggleSidePanel: () => void;
  setKillConfirm: (sessionId: string | null) => void;

  setSessions: (sessions: Session[]) => void;
  addSession: (session: Session) => void;
  removeSession: (sessionId: string) => void;
  updateSessionStatus: (
    sessionId: string,
    status: SessionStatus,
    name?: string | null,
    needsAttentionSince?: number | null
  ) => void;
  updateSessionPrUrl: (sessionId: string, url: string) => void;

  setProjects: (projects: RegisteredProject[]) => void;
  setConfig: (config: Config) => void;

  // Derived
  getSessionByNumber: (number: number) => Session | undefined;
  getNeedsInputSessions: () => Session[];
  getNextNeedsInputSession: (currentId?: string) => Session | undefined;
}

const sortSessions = (sessions: Session[]): Session[] => {
  const priority: Record<SessionStatus, number> = {
    NeedsInput: 0,
    Stuck: 1,
    PrReady: 2,
    Working: 3,
    Done: 4,
    Empty: 5,
  };

  return [...sessions].sort((a, b) => {
    const pa = priority[a.status] ?? 5;
    const pb = priority[b.status] ?? 5;
    if (pa !== pb) return pa - pb;
    if (a.status === "NeedsInput") {
      return (
        (a.needs_attention_since ?? 0) - (b.needs_attention_since ?? 0)
      );
    }
    return a.number - b.number;
  });
};

export const useStore = create<AppState>((set, get) => ({
  viewMode: "overview",
  focusedSessionId: null,
  showNewSessionModal: false,
  showHelpOverlay: false,
  sidePanelVisible: true,
  killConfirmSessionId: null,

  sessions: [],
  projects: [],
  config: null,

  setViewMode: (mode) => set({ viewMode: mode }),

  focusSession: (sessionId) =>
    set({ viewMode: "focus", focusedSessionId: sessionId }),

  returnToOverview: () =>
    set({ viewMode: "overview", focusedSessionId: null }),

  toggleNewSessionModal: () =>
    set((s) => ({ showNewSessionModal: !s.showNewSessionModal })),

  closeNewSessionModal: () => set({ showNewSessionModal: false }),

  toggleHelpOverlay: () =>
    set((s) => ({ showHelpOverlay: !s.showHelpOverlay })),

  toggleSidePanel: () =>
    set((s) => ({ sidePanelVisible: !s.sidePanelVisible })),

  setKillConfirm: (sessionId) => set({ killConfirmSessionId: sessionId }),

  setSessions: (sessions) => set({ sessions: sortSessions(sessions) }),

  addSession: (session) =>
    set((s) => ({ sessions: sortSessions([...s.sessions, session]) })),

  removeSession: (sessionId) =>
    set((s) => ({
      sessions: s.sessions.filter((sess) => sess.id !== sessionId),
      // If the removed session was focused, return to overview
      ...(s.focusedSessionId === sessionId
        ? { viewMode: "overview" as ViewMode, focusedSessionId: null }
        : {}),
    })),

  updateSessionStatus: (sessionId, status, name, needsAttentionSince) =>
    set((s) => ({
      sessions: sortSessions(
        s.sessions.map((sess) => {
          if (sess.id !== sessionId) return sess;
          return {
            ...sess,
            status,
            name: name ?? sess.name,
            needs_attention_since: needsAttentionSince ?? null,
          };
        })
      ),
    })),

  updateSessionPrUrl: (sessionId, url) =>
    set((s) => ({
      sessions: s.sessions.map((sess) =>
        sess.id === sessionId ? { ...sess, pr_url: url } : sess
      ),
    })),

  setProjects: (projects) => set({ projects }),
  setConfig: (config) => set({ config }),

  getSessionByNumber: (number) =>
    get().sessions.find((s) => s.number === number),

  getNeedsInputSessions: () =>
    get().sessions.filter((s) => s.status === "NeedsInput"),

  getNextNeedsInputSession: (currentId) => {
    const needsInput = get()
      .sessions.filter((s) => s.status === "NeedsInput")
      .sort(
        (a, b) =>
          (a.needs_attention_since ?? 0) - (b.needs_attention_since ?? 0)
      );

    if (needsInput.length === 0) return undefined;
    if (!currentId) return needsInput[0];

    const currentIdx = needsInput.findIndex((s) => s.id === currentId);
    if (currentIdx === -1 || currentIdx === needsInput.length - 1) {
      return needsInput[0];
    }
    return needsInput[currentIdx + 1];
  },
}));
