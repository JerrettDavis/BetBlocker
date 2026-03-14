'use client';

import type { HeatmapCell } from '@/lib/api-types';

interface ActivityHeatmapProps {
  data: HeatmapCell[] | undefined;
  isLoading?: boolean;
}

const DAY_LABELS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
const HOURS = Array.from({ length: 24 }, (_, i) => i);

/** Returns a Tailwind bg class based on relative intensity [0–1]. */
function intensityClass(value: number, max: number): string {
  if (max === 0 || value === 0) return 'bg-muted';
  const ratio = value / max;
  if (ratio < 0.15) return 'bg-blue-100 dark:bg-blue-950';
  if (ratio < 0.35) return 'bg-blue-200 dark:bg-blue-900';
  if (ratio < 0.55) return 'bg-blue-400 dark:bg-blue-700';
  if (ratio < 0.75) return 'bg-blue-600 dark:bg-blue-500';
  return 'bg-blue-800 dark:bg-blue-300';
}

export function ActivityHeatmap({ data, isLoading }: ActivityHeatmapProps) {
  if (isLoading) {
    return (
      <div className="flex h-40 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
        Loading heatmap...
      </div>
    );
  }

  if (!data || data.length === 0) {
    return (
      <div className="flex h-40 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
        No activity data available.
      </div>
    );
  }

  // Build lookup: dow -> hour -> count  (ISO dow: 1=Mon … 7=Sun)
  const lookup = new Map<string, number>();
  let max = 0;
  for (const cell of data) {
    lookup.set(`${cell.day_of_week}-${cell.hour_of_day}`, cell.event_count);
    if (cell.event_count > max) max = cell.event_count;
  }

  return (
    <div className="overflow-x-auto">
      <div className="inline-block min-w-full">
        {/* Hour labels */}
        <div className="flex gap-px mb-1 ml-10">
          {HOURS.map((h) => (
            <div
              key={h}
              className="w-5 flex-shrink-0 text-center text-[9px] text-muted-foreground"
            >
              {h % 3 === 0 ? h : ''}
            </div>
          ))}
        </div>

        {/* Day rows */}
        {DAY_LABELS.map((day, idx) => {
          const dow = idx + 1; // ISO day-of-week
          return (
            <div key={day} className="flex items-center gap-px mb-px">
              <span className="w-9 text-right pr-1 text-[10px] text-muted-foreground shrink-0">
                {day}
              </span>
              {HOURS.map((h) => {
                const count = lookup.get(`${dow}-${h}`) ?? 0;
                return (
                  <div
                    key={h}
                    title={`${day} ${h}:00 — ${count} events`}
                    className={`w-5 h-5 flex-shrink-0 rounded-sm ${intensityClass(count, max)}`}
                  />
                );
              })}
            </div>
          );
        })}

        {/* Legend */}
        <div className="flex items-center gap-2 mt-3 text-xs text-muted-foreground">
          <span>Less</span>
          {[0, 0.2, 0.4, 0.6, 0.8, 1].map((v) => (
            <div
              key={v}
              className={`w-4 h-4 rounded-sm ${intensityClass(v * max, max)}`}
            />
          ))}
          <span>More</span>
        </div>
      </div>
    </div>
  );
}
