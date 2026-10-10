/** Observation intent survives session fencing, not explicit cancellation or navigation. */
export function createLiveIntent() {
  let wanted = false;
  let recoveries = 0;
  return {
    get wanted() {
      return wanted;
    },
    start() {
      wanted = true;
      recoveries = 0;
    },
    stop() {
      wanted = false;
      recoveries = 0;
    },
    snapshot() {
      recoveries = 0;
    },
    recover() {
      if (!wanted || recoveries >= 1) return false;
      recoveries++;
      return true;
    },
  };
}
