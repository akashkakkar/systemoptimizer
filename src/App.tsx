import { useState } from "react";
import { AuditLogView } from "./components/AuditLogView";
import { DashboardView } from "./components/DashboardView";

type View = "dashboard" | "audit";

function App() {
  const [view, setView] = useState<View>("dashboard");

  return (
    <div className="min-h-screen bg-gray-950 text-gray-100">
      <nav className="border-b border-gray-800 px-6 py-3 flex gap-4">
        <button
          onClick={() => setView("dashboard")}
          className={`text-sm font-medium transition-colors ${
            view === "dashboard"
              ? "text-blue-400"
              : "text-gray-400 hover:text-gray-200"
          }`}
        >
          Dashboard
        </button>
        <button
          onClick={() => setView("audit")}
          className={`text-sm font-medium transition-colors ${
            view === "audit"
              ? "text-blue-400"
              : "text-gray-400 hover:text-gray-200"
          }`}
        >
          Audit Log
        </button>
      </nav>
      {view === "dashboard" && <DashboardView />}
      {view === "audit" && <AuditLogView />}
    </div>
  );
}

export default App;
