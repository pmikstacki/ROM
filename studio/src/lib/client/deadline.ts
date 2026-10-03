/** Bound waiting even when an injected transport ignores AbortSignal. */
export async function deadline<T>(
  pending: Promise<T>,
  signal: AbortSignal,
  milliseconds: number,
  onTimeout: () => void,
): Promise<T> {
  if (signal.aborted) {
    void pending.catch(() => {});
    throw signal.reason ?? Error("operation aborted");
  }
  let timer: ReturnType<typeof setTimeout> | undefined,
    abort: () => void = () => {};
  const stopped = new Promise<never>((_, reject) => {
    abort = () => reject(signal.reason ?? Error("operation aborted"));
    signal.addEventListener("abort", abort, { once: true });
    timer = setTimeout(() => {
      reject(Error("stream timeout"));
      onTimeout();
    }, milliseconds);
  });
  try {
    return await Promise.race([pending, stopped]);
  } finally {
    clearTimeout(timer);
    signal.removeEventListener("abort", abort);
  }
}
