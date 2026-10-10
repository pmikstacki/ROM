/** Invalid or unavailable time cannot establish unexpired authority. */
export function currentTime(now: () => number): number {
  try {
    const value = now();
    return Number.isFinite(value) && value >= 0 ? value : Infinity;
  } catch {
    return Infinity;
  }
}
