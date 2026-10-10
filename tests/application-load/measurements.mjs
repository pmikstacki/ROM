// Finite scheduled-arrival samples. Never retain request payloads or free-text labels.
const operations = Object.freeze([
  "read",
  "commit",
  "replay",
  "query",
  "reserve",
  "upload",
  "download",
  "stream",
  "work",
]);
const outcomes = Object.freeze([
  "success",
  "denied",
  "overloaded",
  "unknown",
  "timeout",
  "failure",
]);
const fields = ["operation", "planned", "dispatched", "completed", "outcome"];
function quantiles(values) {
  if (!values.length) return { p50: null, p95: null, p99: null };
  const sorted = [...values].sort((a, b) => a - b);
  return Object.fromEntries(
    [50, 95, 99].map((p) => [
      `p${p}`,
      sorted[Math.ceil((sorted.length * p) / 100) - 1],
    ]),
  );
}
export class Measurements {
  #limit;
  #samples = [];
  constructor(limit = 13280) {
    if (!Number.isSafeInteger(limit) || limit < 1 || limit > 13280)
      throw Error("sample budget");
    this.#limit = limit;
  }
  record(value) {
    if (
      !value ||
      Object.keys(value).length !== fields.length ||
      Object.keys(value).some((k) => !fields.includes(k))
    )
      throw Error("measurement fields");
    if (
      !operations.includes(value.operation) ||
      !outcomes.includes(value.outcome)
    )
      throw Error("measurement labels");
    if (
      ![value.planned, value.dispatched, value.completed].every(
        (n) => Number.isFinite(n) && n >= 0 && n <= 1200000,
      ) ||
      value.dispatched < value.planned ||
      value.completed < value.dispatched
    )
      throw Error("measurement timing");
    if (this.#samples.length >= this.#limit) throw Error("sample budget");
    this.#samples.push(Object.freeze({ ...value }));
  }
  summary() {
    return Object.fromEntries(
      operations.map((operation) => {
        const values = this.#samples.filter((s) => s.operation === operation);
        return [
          operation,
          {
            count: values.length,
            outcomes: Object.fromEntries(
              outcomes.map((outcome) => [
                outcome,
                values.filter((s) => s.outcome === outcome).length,
              ]),
            ),
            scheduled: quantiles(values.map((s) => s.completed - s.planned)),
            dispatched: quantiles(
              values.map((s) => s.completed - s.dispatched),
            ),
            generator_delay: quantiles(
              values.map((s) => s.dispatched - s.planned),
            ),
          },
        ];
      }),
    );
  }
}
