// Keep the deadline and body reader owned until consumption completes.
export async function withDeadline(milliseconds, signals, operation) {
  const controller = new AbortController(), forwarding = [];
  const timer = setTimeout(
    () => controller.abort(new DOMException("Operation timed out", "TimeoutError")),
    milliseconds,
  );
  try {
    for (const source of signals) {
      const abort = () => controller.abort(source.reason);
      if (source.aborted) abort();
      else {
        source.addEventListener("abort", abort, { once: true });
        forwarding.push([source, abort]);
      }
    }
    return await operation(controller.signal);
  } finally {
    clearTimeout(timer);
    for (const [source, abort] of forwarding)
      source.removeEventListener("abort", abort);
  }
}

export async function* signalBody(body, signal) {
  const reader = body.getReader(),
    abort = () => { reader.cancel(signal.reason).catch(() => {}); };
  signal.addEventListener("abort", abort, { once: true });
  try {
    signal.throwIfAborted();
    while (true) {
      const chunk = await reader.read();
      signal.throwIfAborted();
      if (chunk.done) return;
      yield chunk.value;
    }
  } finally {
    signal.removeEventListener("abort", abort);
    await reader.cancel().catch(() => {});
    reader.releaseLock();
  }
}
