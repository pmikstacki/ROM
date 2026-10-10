// Both observations must carry their caller's finite browser deadlines.
// A navigation completion alone does not establish form readiness or a callback.
export function awaitLoginDispatch(formVisible, callbackResponse) {
  return Promise.any([
    formVisible.then(() => ({ kind: 'form' })),
    callbackResponse.then(response => ({ kind: 'callback', response })),
  ]);
}
