import { useState, useEffect, useRef, useMemo } from "react";
import Fuse from "fuse.js";
import { useStore } from "../store";
import {
  listProjects,
  createSession,
  addProject as addProjectApi,
} from "../lib/tauri";
import type { RegisteredProject, AgentType } from "../types";

type ModalStep = "select-project" | "confirm";

export default function NewSessionModal() {
  const { showNewSessionModal, closeNewSessionModal, addSession, setProjects } =
    useStore();

  const [step, setStep] = useState<ModalStep>("select-project");
  const [projects, setLocalProjects] = useState<RegisteredProject[]>([]);
  const [search, setSearch] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [selectedProject, setSelectedProject] =
    useState<RegisteredProject | null>(null);
  const [branchName, setBranchName] = useState("");
  const [editingBranch, setEditingBranch] = useState(false);
  const [agent, setAgent] = useState<AgentType>("Claude");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [addingProject, setAddingProject] = useState(false);
  const [newProjectPath, setNewProjectPath] = useState("");

  const searchRef = useRef<HTMLInputElement>(null);
  const branchRef = useRef<HTMLInputElement>(null);
  const newProjectRef = useRef<HTMLInputElement>(null);
  const overlayRef = useRef<HTMLDivElement>(null);

  // Load projects
  useEffect(() => {
    if (showNewSessionModal) {
      listProjects().then((p) => {
        setLocalProjects(p);
        setProjects(p);
      });
      setStep("select-project");
      setSearch("");
      setSelectedIndex(0);
      setSelectedProject(null);
      setEditingBranch(false);
      setAgent("Claude");
      setError(null);
      setAddingProject(false);
      setNewProjectPath("");
    }
  }, [showNewSessionModal]);

  // Focus appropriate element based on step
  useEffect(() => {
    if (showNewSessionModal && step === "select-project") {
      setTimeout(() => searchRef.current?.focus(), 50);
    } else if (showNewSessionModal && step === "confirm") {
      setTimeout(() => overlayRef.current?.focus(), 50);
    }
  }, [showNewSessionModal, step]);

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
    if (!search.trim()) return projects;
    return fuse.search(search).map((r) => r.item);
  }, [search, projects, fuse]);

  // Clamp selected index
  useEffect(() => {
    if (selectedIndex >= filteredProjects.length) {
      setSelectedIndex(Math.max(0, filteredProjects.length - 1));
    }
  }, [filteredProjects.length]);

  if (!showNewSessionModal) return null;

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      if (addingProject) {
        setAddingProject(false);
        return;
      }
      if (editingBranch) {
        setEditingBranch(false);
        return;
      }
      if (step === "confirm") {
        setStep("select-project");
        return;
      }
      closeNewSessionModal();
      return;
    }

    if (step === "select-project" && !addingProject) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) =>
          Math.min(i + 1, filteredProjects.length - 1)
        );
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter" && filteredProjects.length > 0) {
        e.preventDefault();
        selectProject(filteredProjects[selectedIndex]);
      } else if (e.key === "+") {
        e.preventDefault();
        setAddingProject(true);
        setTimeout(() => newProjectRef.current?.focus(), 50);
      }
    } else if (step === "confirm" && !editingBranch) {
      if (e.key === "Enter") {
        e.preventDefault();
        handleLaunch();
      } else if (e.key === "e") {
        e.preventDefault();
        setEditingBranch(true);
        setTimeout(() => branchRef.current?.focus(), 50);
      } else if (e.key === "a") {
        e.preventDefault();
        cycleAgent();
      }
    }
  };

  const selectProject = (project: RegisteredProject) => {
    setSelectedProject(project);
    // Generate default branch name
    const sessions = useStore.getState().sessions;
    const nextNum =
      sessions.length > 0
        ? Math.max(...sessions.map((s) => s.number)) + 1
        : 1;
    setBranchName(`cmux/session-${nextNum}`);
    setStep("confirm");
  };

  const cycleAgent = () => {
    setAgent((a) => {
      if (a === "Claude") return "Codex";
      if (a === "Codex") return "Shell";
      return "Claude";
    });
  };

  const handleLaunch = async () => {
    if (!selectedProject || loading) return;
    setLoading(true);
    setError(null);

    try {
      const session = await createSession(
        selectedProject.path,
        agent,
        branchName || undefined
      );
      addSession(session);
      closeNewSessionModal();
      // Switch to focus mode for the new session
      useStore.getState().focusSession(session.id);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleAddProject = async () => {
    if (!newProjectPath.trim()) return;
    try {
      const project = await addProjectApi(newProjectPath.trim());
      setLocalProjects((prev) => [project, ...prev]);
      setProjects([project, ...projects]);
      setAddingProject(false);
      setNewProjectPath("");
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div
      className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 outline-none"
      onKeyDown={handleKeyDown}
      tabIndex={-1}
      ref={overlayRef}
    >
      <div className="bg-cmux-surface border border-cmux-border rounded-lg w-[400px] max-h-[500px] overflow-hidden shadow-2xl">
        {step === "select-project" && (
          <>
            <div className="p-4 border-b border-cmux-border">
              <h2 className="text-[13px] font-semibold text-cmux-text-primary mb-3">
                New Session
              </h2>
              {addingProject ? (
                <div className="flex gap-2">
                  <input
                    ref={newProjectRef}
                    type="text"
                    value={newProjectPath}
                    onChange={(e) => setNewProjectPath(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.preventDefault();
                        handleAddProject();
                      }
                    }}
                    placeholder="/path/to/project"
                    className="flex-1 bg-cmux-bg border border-cmux-border rounded px-2 py-1 text-[12px] text-cmux-text-primary font-mono focus:outline-none focus:border-cmux-working"
                  />
                  <button
                    onClick={handleAddProject}
                    className="px-2 py-1 bg-cmux-working text-white rounded text-[11px] hover:bg-blue-600"
                  >
                    Add
                  </button>
                </div>
              ) : (
                <input
                  ref={searchRef}
                  type="text"
                  value={search}
                  onChange={(e) => {
                    setSearch(e.target.value);
                    setSelectedIndex(0);
                  }}
                  placeholder="Search projects..."
                  className="w-full bg-cmux-bg border border-cmux-border rounded px-2 py-1.5 text-[12px] text-cmux-text-primary font-mono focus:outline-none focus:border-cmux-working"
                />
              )}
            </div>
            <div className="max-h-[300px] overflow-y-auto">
              {filteredProjects.length === 0 ? (
                <div className="p-4 text-center text-cmux-text-muted text-[11px]">
                  No projects found. Press{" "}
                  <kbd className="px-1 py-0.5 rounded bg-cmux-border text-[9px]">
                    +
                  </kbd>{" "}
                  to add one.
                </div>
              ) : (
                filteredProjects.map((project, i) => (
                  <button
                    key={project.path}
                    onClick={() => selectProject(project)}
                    className={`w-full text-left px-4 py-2 flex items-center justify-between transition-colors ${
                      i === selectedIndex
                        ? "bg-cmux-border"
                        : "hover:bg-cmux-bg"
                    }`}
                  >
                    <span className="text-[12px] text-cmux-text-primary font-semibold">
                      {project.name}
                    </span>
                    <span className="text-[10px] text-cmux-text-muted truncate ml-2 max-w-[180px]">
                      {project.path}
                    </span>
                  </button>
                ))
              )}
            </div>
            <div className="px-4 py-2 border-t border-cmux-border text-[10px] text-cmux-text-muted flex gap-3">
              <span>
                <kbd className="px-1 py-0.5 rounded bg-cmux-border text-[9px]">
                  enter
                </kbd>{" "}
                select
              </span>
              <span>
                <kbd className="px-1 py-0.5 rounded bg-cmux-border text-[9px]">
                  esc
                </kbd>{" "}
                cancel
              </span>
              <span>
                <kbd className="px-1 py-0.5 rounded bg-cmux-border text-[9px]">
                  +
                </kbd>{" "}
                add project
              </span>
            </div>
          </>
        )}

        {step === "confirm" && selectedProject && (
          <>
            <div className="p-4 border-b border-cmux-border">
              <h2 className="text-[13px] font-semibold text-cmux-text-primary">
                New Session: {selectedProject.name}
              </h2>
            </div>
            <div className="p-4 space-y-3">
              <div className="flex items-center justify-between text-[12px]">
                <span className="text-cmux-text-muted">Branch:</span>
                {editingBranch ? (
                  <input
                    ref={branchRef}
                    type="text"
                    value={branchName}
                    onChange={(e) => setBranchName(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.preventDefault();
                        setEditingBranch(false);
                      }
                    }}
                    onBlur={() => setEditingBranch(false)}
                    className="bg-cmux-bg border border-cmux-border rounded px-2 py-0.5 text-[11px] text-cmux-text-primary font-mono focus:outline-none focus:border-cmux-working w-[200px]"
                  />
                ) : (
                  <span className="text-cmux-text-secondary font-mono">
                    {branchName}
                  </span>
                )}
              </div>
              <div className="flex items-center justify-between text-[12px]">
                <span className="text-cmux-text-muted">Agent:</span>
                <span className="text-cmux-text-secondary">
                  {agent.toLowerCase()}
                </span>
              </div>
              <div className="flex items-center justify-between text-[12px]">
                <span className="text-cmux-text-muted">Dir:</span>
                <span className="text-cmux-text-secondary font-mono text-[10px] truncate max-w-[200px]">
                  {selectedProject.path}
                </span>
              </div>
              {error && (
                <div className="text-cmux-stuck text-[11px] bg-red-900/20 px-2 py-1 rounded">
                  {error}
                </div>
              )}
            </div>
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
                  e
                </kbd>{" "}
                edit branch
              </span>
              <span>
                <kbd className="px-1 py-0.5 rounded bg-cmux-border text-[9px]">
                  a
                </kbd>{" "}
                change agent
              </span>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
