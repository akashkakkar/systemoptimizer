import type { ProcessInfo } from "../lib/tauri";

interface ProcessTableProps {
  processes: ProcessInfo[];
}

export function ProcessTable({ processes }: ProcessTableProps) {
  const top = processes.slice(0, 20);

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-5">
      <h3 className="text-lg font-semibold text-gray-100 mb-4">
        Processes (top {top.length})
      </h3>

      <div className="overflow-x-auto">
        <table className="w-full text-sm text-left">
          <thead className="text-gray-400 border-b border-gray-700">
            <tr>
              <th className="py-2 pr-4">PID</th>
              <th className="py-2 pr-4">Name</th>
              <th className="py-2 pr-4 text-right">CPU %</th>
              <th className="py-2 pr-4 text-right">Mem %</th>
              <th className="py-2">Status</th>
            </tr>
          </thead>
          <tbody>
            {top.map((proc) => (
              <tr
                key={proc.pid}
                className="border-b border-gray-800 hover:bg-gray-800/50"
              >
                <td className="py-2 pr-4 text-gray-400 font-mono">
                  {proc.pid}
                </td>
                <td className="py-2 pr-4 text-gray-200 truncate max-w-[200px]">
                  {proc.name}
                </td>
                <td className="py-2 pr-4 text-right font-mono">
                  <span
                    className={
                      proc.cpu_percent > 80
                        ? "text-red-400"
                        : proc.cpu_percent > 50
                          ? "text-yellow-400"
                          : "text-gray-300"
                    }
                  >
                    {proc.cpu_percent.toFixed(1)}
                  </span>
                </td>
                <td className="py-2 pr-4 text-right font-mono">
                  <span
                    className={
                      proc.memory_percent > 20
                        ? "text-yellow-400"
                        : "text-gray-300"
                    }
                  >
                    {proc.memory_percent.toFixed(1)}
                  </span>
                </td>
                <td className="py-2">
                  <span
                    className={`px-2 py-0.5 rounded text-xs ${
                      proc.status === "running"
                        ? "bg-green-900/50 text-green-300"
                        : proc.status === "zombie"
                          ? "bg-red-900/50 text-red-300"
                          : "bg-gray-800 text-gray-400"
                    }`}
                  >
                    {proc.status}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {processes.length === 0 && (
        <p className="text-gray-500 text-sm py-4 text-center">
          Process data not available on this platform.
        </p>
      )}
    </div>
  );
}
