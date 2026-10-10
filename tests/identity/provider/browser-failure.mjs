// Evidence contains only a closed classification, never a browser message or stack.
export function classifyBrowserFailure(error) {
  const message = error instanceof Error ? error.message : '';
  const kind = /ReferenceError:/.test(message) ? 'reference-error'
    : /Execution context was destroyed|Target page, context or browser has been closed/.test(message) ? 'context-lost'
    : /Failed to fetch|Load failed|NetworkError|Network request failed/.test(message) ? 'network-failure'
    : error?.name === 'TimeoutError' ? 'deadline'
    : /TypeError:/.test(message) ? 'type-error' : 'other';
  return { kind };
}
