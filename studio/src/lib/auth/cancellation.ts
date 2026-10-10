/** Stop waiting even if a host driver ignores its signal. */
export async function untilAborted<T>(
  pending: Promise<T>,
  signal: AbortSignal,
): Promise<T> {
  void pending.catch(() => {});
  if (signal.aborted) throw Error("session cancelled");
  let stop: () => void = () => {};
  const aborted = new Promise<never>((_, reject) => {
    stop = () => reject(Error("session cancelled"));
    signal.addEventListener("abort", stop, { once: true });
  });
  try {
    return await Promise.race([pending, aborted]);
  } finally {
    signal.removeEventListener("abort", stop);
  }
}
