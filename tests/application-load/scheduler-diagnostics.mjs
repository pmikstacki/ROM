// Fixed group metadata only. Request payloads and free-text labels never enter evidence.
const categories = ["read", "mutation", "replay", "blob", "other"];
const edges = [1, 10, 100, 1000, 5000, 1200000];
const histogram = () => ({
  count: 0,
  sum_ms: 0,
  max_ms: 0,
  buckets: [0, 0, 0, 0, 0, 0],
});
const indexValid = (value) =>
  Number.isSafeInteger(value) && value >= 0 && value < 12000;
const timeValid = (value) =>
  Number.isFinite(value) && value >= 0 && value <= 1200000;

export class SchedulerDiagnostics {
  #activeLimit;
  #queued = new Map();
  #d = {
    schema: "rom-load-scheduler-diagnostics-v1",
    scope: "client offers and dispatch; not accepted ROM Work",
    overflowed: false,
    refused_idle_slots: 0,
    refused_occupied_slots: 0,
    due_batch_max: 0,
    refused_category: Object.fromEntries(categories.map((key) => [key, 0])),
    offered_lateness: histogram(),
    queue_wait: histogram(),
    continuations_skipped_non_success: 0,
    requests_cancelled_before_execution: 0,
  };
  constructor(activeLimit) {
    if (
      !Number.isSafeInteger(activeLimit) ||
      activeLimit < 1 ||
      activeLimit > 32
    ) {
      this.#d.overflowed = true;
      this.#activeLimit = 1;
    } else this.#activeLimit = activeLimit;
  }
  #add(object, key, count) {
    if (!Number.isSafeInteger(count) || count < 0) {
      this.#d.overflowed = true;
      return;
    }
    const value = object[key] + count;
    if (value > 13280) {
      this.#d.overflowed = true;
      object[key] = 13280;
    } else object[key] = value;
  }
  #sample(histogram, value) {
    if (!timeValid(value) || histogram.count === 12000) {
      this.#d.overflowed = true;
      return;
    }
    histogram.count++;
    histogram.sum_ms += value;
    histogram.max_ms = Math.max(histogram.max_ms, value);
    histogram.buckets[edges.findIndex((edge) => value <= edge)]++;
  }
  offer(index, category, planned, offered, accepted, active) {
    if (
      !indexValid(index) ||
      !timeValid(planned) ||
      !timeValid(offered) ||
      offered < planned ||
      typeof accepted !== "boolean" ||
      !Number.isSafeInteger(active) ||
      active < 0 ||
      active > this.#activeLimit
    ) {
      this.#d.overflowed = true;
      return;
    }
    this.#sample(this.#d.offered_lateness, offered - planned);
    if (!accepted) {
      this.#add(
        this.#d,
        active < this.#activeLimit
          ? "refused_idle_slots"
          : "refused_occupied_slots",
        1,
      );
      this.#add(
        this.#d.refused_category,
        categories.includes(category) ? category : "other",
        1,
      );
      return;
    }
    if (this.#queued.size === 64 || this.#queued.has(index)) {
      this.#d.overflowed = true;
      return;
    }
    this.#queued.set(index, offered);
  }
  dueBatch(count) {
    if (!Number.isSafeInteger(count) || count < 0 || count > 12000) {
      this.#d.overflowed = true;
      return;
    }
    this.#d.due_batch_max = Math.max(this.#d.due_batch_max, count);
  }
  dispatch(index, dispatched) {
    const offered = this.#queued.get(index);
    if (
      !indexValid(index) ||
      !timeValid(dispatched) ||
      offered === undefined ||
      dispatched < offered
    ) {
      this.#d.overflowed = true;
      return;
    }
    this.#queued.delete(index);
    this.#sample(this.#d.queue_wait, dispatched - offered);
  }
  nonSuccess(remaining) {
    this.#add(this.#d, "continuations_skipped_non_success", remaining);
  }
  cancelled(remaining) {
    this.#add(this.#d, "requests_cancelled_before_execution", remaining);
  }
  snapshot() {
    return {
      ...this.#d,
      refused_category: { ...this.#d.refused_category },
      offered_lateness: {
        ...this.#d.offered_lateness,
        buckets: [...this.#d.offered_lateness.buckets],
      },
      queue_wait: {
        ...this.#d.queue_wait,
        buckets: [...this.#d.queue_wait.buckets],
      },
      histogram_upper_ms: [...edges],
      pending_offer_times: this.#queued.size,
    };
  }
}
