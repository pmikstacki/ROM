// Failure diagnostics must remain available when the artifact filesystem is full.
const tail = value => Buffer.from(String(value ?? '')).subarray(-8190).toString('utf8');
export const persistenceError = error => ({ code: error.code ?? null, message: tail(error.message) });
export function reportEvidenceFailure(type, details) {
  console.error(JSON.stringify({ type, ...details }));
}
export function reportCommandEvidenceFailure(command, outcome, error) {
  reportEvidenceFailure('release-command-evidence-failure', {
    command, persistence_error: persistenceError(error),
    stdout_tail: tail(outcome.stdout), stderr_tail: tail(outcome.stderr),
  });
}
