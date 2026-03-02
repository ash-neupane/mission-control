import { useStore } from "./store";
import { useSessionInit } from "./hooks/useSession";
import { useHotkeys } from "./hooks/useHotkeys";
import TitleBar from "./components/TitleBar";
import StatusBar from "./components/StatusBar";
import Overview from "./components/Overview";
import FocusMode from "./components/FocusMode";
import NewSessionModal from "./components/NewSessionModal";
import HelpOverlay from "./components/HelpOverlay";
import KillConfirmDialog from "./components/KillConfirmDialog";

export default function App() {
  const { viewMode } = useStore();

  // Initialize session data and event listeners
  useSessionInit();

  // Register global hotkeys
  useHotkeys();

  return (
    <div className="flex flex-col h-screen bg-cmux-bg overflow-hidden">
      <TitleBar />
      {viewMode === "overview" ? <Overview /> : <FocusMode />}
      <StatusBar />
      <NewSessionModal />
      <HelpOverlay />
      <KillConfirmDialog />
    </div>
  );
}
