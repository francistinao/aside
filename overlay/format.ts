// Clock text for the footer: "0:58", "12:05".
export function formatClock(seconds: number): string {
  const whole = Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
  const minutes = Math.floor(whole / 60);
  const rest = whole % 60;
  return `${minutes}:${rest < 10 ? '0' : ''}${rest}`;
}

export function formatProgress(elapsed: number, total: number): string {
  return `${formatClock(elapsed)} / ${formatClock(total)}`;
}
