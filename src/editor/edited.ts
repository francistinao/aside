// "Edited 2 min ago" for the toolbar. Refreshed once a minute by the caller.

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

export function formatEdited(modifiedMs: number, nowMs: number): string {
  const age = Math.max(0, nowMs - modifiedMs);
  if (age < MINUTE) return 'Edited just now';
  if (age < HOUR) {
    const minutes = Math.floor(age / MINUTE);
    return `Edited ${minutes} min ago`;
  }
  if (age < DAY) {
    const hours = Math.floor(age / HOUR);
    return `Edited ${hours} ${hours === 1 ? 'hour' : 'hours'} ago`;
  }
  const days = Math.floor(age / DAY);
  if (days === 1) return 'Edited yesterday';
  if (days < 7) return `Edited ${days} days ago`;
  return `Edited ${new Date(modifiedMs).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}`;
}
