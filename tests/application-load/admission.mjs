// Client-side admission only. This does not claim ownership of accepted ROM work.
export class Admission {
  #limits;
  #offered = 0;
  #rejected = 0;
  #queue = [];
  #active = new Set();
  #maximumActive = 0;
  #maximumPending = 0;
  constructor(limits) {
    if (
      !limits ||
      Object.keys(limits).length !== 3 ||
      Object.keys(limits).some(
        (k) => !["active", "pending", "scheduled"].includes(k),
      )
    )
      throw Error("admission limits");
    for (const [key, maximum] of Object.entries({
      active: 32,
      pending: 64,
      scheduled: 12000,
    })) {
      if (
        !Number.isSafeInteger(limits[key]) ||
        limits[key] < 1 ||
        limits[key] > maximum
      )
        throw Error("admission limits");
    }
    this.#limits = { ...limits };
  }
  offer(value) {
    if (
      !value ||
      Object.keys(value).length !== 2 ||
      Object.keys(value).some((k) => !["index", "planned"].includes(k)) ||
      value.index !== this.#offered ||
      !Number.isFinite(value.planned) ||
      value.planned < 0 ||
      value.planned > 120000
    )
      throw Error("scheduled operation identity");
    if (this.#offered >= this.#limits.scheduled)
      throw Error("scheduled operation budget");
    this.#offered++;
    if (this.#queue.length === this.#limits.pending) {
      this.#rejected++;
      return false;
    }
    this.#queue.push(Object.freeze({ ...value }));
    this.#maximumPending = Math.max(this.#maximumPending, this.#queue.length);
    return true;
  }
  take() {
    if (this.#active.size === this.#limits.active || !this.#queue.length)
      return null;
    const value = this.#queue.shift();
    this.#active.add(value.index);
    this.#maximumActive = Math.max(this.#maximumActive, this.#active.size);
    return value;
  }
  finish(index) {
    if (!this.#active.delete(index)) throw Error("unknown active operation");
  }
  stats() {
    return {
      scheduled: this.#offered,
      rejected_scheduling: this.#rejected,
      active: this.#active.size,
      pending: this.#queue.length,
      maximum_active: this.#maximumActive,
      maximum_pending: this.#maximumPending,
    };
  }
}
