import { useState, useEffect, useRef, useMemo } from "react";
import Fuse from "fuse.js";
import { useStore } from "../store";
import {
  listProjects,
  createSession,
  addProject as addProjectApi,
} from "../lib/tauri";
import type { RegisteredProject, AgentType } from "../types";

export default function NewSessionModal() {
  const { showNewSessionModal, closeNewSessionModal, addSession, setProjects } =
    useStore();

  const [projects, setLocalProjects] = useState<RegisteredProject[]>([]);
  const [search, setSearch] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [branchName, setBranchName] = useState("");
  const [editingBranch, setEditingBranch] = useState(false);
  const [agent, setAgent] = useState<AgentType>("Claude");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const searchRef = useRef<HTMLInputElement>(null);
  const branchRef = useRef<HTMLInputElement>(null);

  // Detect if search input looks like a file path (add-project intent)
  const isPathInput = search.startsWith("/") || search.startsWith("~");

  // Load projects + reset state when modal opens
  useEffect(() => {
    if (showNewSessionModal) {
      listProjects()
        .then((p) => {
          setLocalProjects(p);
          setProjects(p);
        })
        .catch((err) => setError(String(err)));
      setSearch("");
      setSelectedIndex(0);
      setEditingBranch(false);
      setAgent("Claude");
      setError(null);
      setLoading(false);
      updateBranchDefault();
    }
  }, [showNewSessionModal]);

  // Auto-focus search on open
  useEffect(() => {
    if (!showNewSessionModal) return;
    const id = setTimeout(() => searchRef.current?.focus(), 50);
    return () => clearTimeout(id);
  }, [showNewSessionModal]);

  // Fuzzy search
  const fuse = useMemo(
    () =>
      new Fuse(projects, {
        keys: ["name", "path"],
        threshold: 0.4,
      }),
    [projects]
  );

  const filteredProjects = useMemo(() => {
    if (isPathInput) return []; // path mode — don't show project list
    if (!search.trim()) return projects;
    return fuse.search(search).map((r) => r.item);
  }, [search, projects, fuse, isPathInput]);

  // Clamp selected index
  useEffect(() => {
    if (selectedIndex >= filteredProjects.length) {
      setSelectedIndex(Math.max(0, filteredProjects.length - 1));
    }
  }, [filteredProjects.length]);

  if (!showNewSessionModal) return null;

  function updateBranchDefault() {
    const sessions = useStore.getState().sessions;
    const nextNum =
      sessions.length > 0
        ? Math.max(...sessions.map((s) => s.number)) + 1
        : 1;
    setBranchName(`cmux/session-${nextNum}`);
  }

  const selectedProject =
    filteredProjects.length > 0
      ? filteredProjects[Math.min(selectedIndex, filteredProjects.length - 1)]
      : null;

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      if (editingBranch) {
        setEditingBranch(false);
        return;
      }
      closeNewSessionModal();
      return;
    }

    if (editingBranch) return; // let branch input handle its own keys

    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) =>
        Math.min(i + 1, filteredProjects.length - 1)
      );
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (isPathInput) {
        handleAddAndLaunch();
      } else if (selectedProject) {
        handleLaunch(selectedProject);
      }
    } else if (e.key === "Tab" && !e.shiftKey) {
      // Tab to focus the branch/agent area without leaving the modal
      e.preventDefault();
      setEditingBranch(true);
      setTimeout(() => branchRef.current?.focus(), 50);
    }
  };

  const cycleAgent = () => {
    setAgent((a) => {
      if (a === "Claude") return "Codex";
      if (a === "Codex") return "Shell";
      return "Claude";
    });
  };

  const handleLaunch = async (project: RegisteredProject) => {
    if (loading) return;
    setLoading(true);
    setError(null);

    try {
      const session = await createSession(
        project.path,
        agent,
        branchName || undefined
      );
      addSession(session);
      closeNewSessionModal();
      useStore.getState().focusSession(session.id);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleAddAndLaunch = async () => {
    const path = search.trim();
    if (!path || loading) return;
    setLoading(true);
    setError(null);

    try {
      // Register the project first
      const project = await addProjectApi(path);
      setLocalProjects((prev) => [project, ...prev]);
      const currentProjects = useStore.getState().projects;
      setProjects([project, ...currentProjects]);

      // Then launch a session on it
      const session = await createSession(
        project.path,
        agent,
        branchName || undefined
      );
      addSession(session);
      closeNewSessionModal();
      useStore.getState().focusSession(session.id);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const hasProjects = projects.length > 0;

  return (
    <div
      className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 outline-none"
      onKeyDown={handleKeyDown}
      tabIndex={-1}
    >
      <div className="bg-cmux-surface border border-cmux-border rounded-lg w-[420px] max-h-[520px] overflow-hidden shadow-2xl">
        {/* Header + search */}
        <div className="p-4 pb-3 border-b border-cmux-border">
          <h2 className="text-[13px] font-semibold text-cmux-text-primary mb-3">
            New Session
          </h2>
          <input
            ref={searchRef}
            type="text"
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setSelectedIndex(0);
            }}
            placeholder={
              hasProjects
                ? "Search projects or paste a path..."
                : "Paste a project path to get started..."
            }
            className="w-full bg-cmux-bg border border-cmux-border rounded px-2 py-1.5 text-[12px] text-cmux-text-primary font-mono focus:outline-none focus:border-cmux-working"
          />
        </div>

        {/* Project list OR path-add prompt */}
        <div className="max-h-[260px] overflow-y-auto">
          {isPathInput ? (
            <div className="p-4">
              <div className="flex items-center gap-2 text-[11px] text-cmux-text-secondary mb-2">
                <span className="w-1.5 h-1.5 rounded-full bg-cmux-working flex-shrink-0" />
                <span className="font-mono truncate">{search}</span>
              </div>
              <p className="text-[10px] text-cmux-text-muted">
                Press{" "}
                <kbd className="px-1 py-0.5 rounded bg-cmux-border text-[9px]">
                  enter
                </kbd>{" "}
                to add this project and launch a session
              </p>
            </div>
          ) : filteredProjects.length === 0 ? (
            <div className="p-4 text-center">
              <p className="text-cmux-text-muted text-[11px] mb-1">
                {hasProjects
                  ? "No matches"
                  : "No projects registered yet"}
              </p>
              <p className="text-cmux-text-muted text-[10px]">
                Type a{" "}
                <span className="text-cmux-text-secondary font-mono">/path</span>{" "}
                to add a project
              </p>
            </div>
          ) : (
            filteredProjects.map((project, i) => (
              <button
                key={project.path}
                onClick={() => handleLaunch(project)}
                className={`w-full text-left px-4 py-2 flex items-center justify-between transition-colors ${
                  i === selectedIndex
                    ? "bg-cmux-border"
                    : "hover:bg-cmux-bg"
                }`}
              >
                <span className="text-[12px] text-cmux-text-primary font-semibold">
                  {project.name}
                </span>
                <span className="text-[10px] text-cmux-text-muted truncate ml-2 max-w-[200px]">
                  {project.path}
                </span>
              </button>
            ))
          )}
        </div>

        {/* Inline config (branch + agent) — always visible, not a separate step */}
        <div className="px-4 py-2 border-t border-cmux-border space-y-1.5">
          <div className="flex items-center justify-between text-[11px]">
            <span className="text-cmux-text-muted">branch</span>
            {editingBranch ? (
              <input
                ref={branchRef}
                type="text"
                value={branchName}
                onChange={(e) => setBranchName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    e.stopPropagation();
                    setEditingBranch(false);
                    searchRef.current?.focus();
                  }
                  if (e.key === "Tab") {
                    e.preventDefault();
                    e.stopPropagation();
                    setEditingBranch(false);
                    searchRef.current?.focus();
                  }
                }}
                onBlur={() => setEditingBranch(false)}
                className="bg-cmux-bg border border-cmux-border rounded px-1.5 py-0.5 text-[10px] text-cmux-text-primary font-mono focus:outline-none focus:border-cmux-working w-[200px] text-right"
              />
            ) : (
              <button
                onClick={() => {
                  setEditingBranch(true);
                  setTimeout(() => branchRef.current?.focus(), 50);
                }}
                className="text-cmux-text-secondary font-mono text-[10px] hover:text-cmux-text-primary transition-colors"
              >
                {branchName}
              </button>
            )}
          </div>
          <div className="flex items-center justify-between text-[11px]">
            <span className="text-cmux-text-muted">agent</span>
            <button
              onClick={cycleAgent}
              className="text-cmux-text-secondary text-[10px] hover:text-cmux-text-primary transition-colors"
            >
              {agent.toLowerCase()}
            </button>
          </div>
        </div>

        {/* Error display */}
        {error && (
          <div className="mx-4 mb-2 text-cmux-stuck text-[11px] bg-red-900/20 px-2 py-1 rounded">
            {error}
          </div>
        )}

        {/* Footer hints */}
        <div className="px-4 py-2 border-t border-cmux-border text-[10px] text-cmux-text-muted flex gap-3">
          <span>
            <kbd className="px-1 py-0.5 rounded bg-cmux-border text-[9px]">
              enter
            </kbd>{" "}
            {loading ? "launching..." : "launch"}
          </span>
          <span>
            <kbd className="px-1 py-0.5 rounded bg-cmux-border text-[9px]">
              esc
            </kbd>{" "}
            cancel
          </span>
          <span>
            <kbd className="px-1 py-0.5 rounded bg-cmux-border text-[9px]">
              tab
            </kbd>{" "}
            edit branch
          </span>
        </div>
      </div>
    </div>
  );
}
