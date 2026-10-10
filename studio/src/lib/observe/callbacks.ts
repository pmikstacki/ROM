/** Consume host callback failures without delaying or taking lifecycle ownership. */
export function callObservationCallback(
  callback: () => unknown,
  onFailure: () => void,
): void {
  try {
    const result = callback();
    if (
      result !== null &&
      (typeof result === "object" || typeof result === "function")
    ) {
      void Promise.resolve(result).catch(onFailure);
    }
  } catch {
    onFailure();
  }
}
