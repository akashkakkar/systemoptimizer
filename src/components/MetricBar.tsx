interface MetricBarProps {
  percent: number;
  label: string;
}

function getBarColor(percent: number): string {
  if (percent >= 90) return "bg-red-500";
  if (percent >= 70) return "bg-yellow-500";
  return "bg-green-500";
}

function getStatusLabel(percent: number): string {
  if (percent >= 90) return "Critical";
  if (percent >= 70) return "Warning";
  return "Healthy";
}

export function MetricBar({ percent, label }: MetricBarProps) {
  const clamped = Math.min(100, Math.max(0, percent));
  const color = getBarColor(clamped);
  const status = getStatusLabel(clamped);

  return (
    <div className="w-full">
      <div className="flex justify-between text-sm mb-1">
        <span className="text-gray-300">{label}</span>
        <span className="text-gray-400">
          {clamped.toFixed(1)}% — {status}
        </span>
      </div>
      <div
        className="w-full h-3 bg-gray-700 rounded-full overflow-hidden"
        role="progressbar"
        aria-valuenow={clamped}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={`${label}: ${clamped.toFixed(1)}% used, ${status}`}
      >
        <div
          className={`h-full rounded-full transition-all duration-300 ${color}`}
          style={{ width: `${clamped}%` }}
        />
      </div>
    </div>
  );
}
