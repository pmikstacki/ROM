// Own only the code/token handoff. No code, cookie, credential or token is logged.
export class NativeTlsHandoff {
  #route; #held = false; #drained = false; #state = 'preparing';
  constructor(route) {
    if (!['token', 'jwks'].includes(route)) throw Error('closed acquisition route required');
    this.#route = route;
  }
  authorizationHeld() {
    if (this.#state !== 'preparing' || this.#held) throw Error('authorization handoff already consumed');
    this.#held = true;
  }
  browserDrained() {
    if (this.#state !== 'preparing' || this.#drained) throw Error('browser drain already observed');
    this.#drained = true;
  }
  arm() {
    if (this.#state !== 'preparing' || !this.#held || !this.#drained) throw Error('held authorization and physical browser drain required');
    this.#state = this.#route === 'token' ? 'token-rotated' : 'waiting-token';
    return this.#route === 'token' ? 'rotate-before-callback' : 'wait-for-token';
  }
  tokenForwarded(status) {
    if (this.#state !== 'waiting-token' || status !== 200) throw Error('original successful token response required');
    this.#state = 'closing-token-connection';
    return 'rotate-and-close-native-connection';
  }
  nativeConnectionClosed() {
    if (this.#state !== 'closing-token-connection') throw Error('native token connection closure is out of sequence');
    this.#state = 'jwks-armed';
  }
  beforeJwks() {
    if (this.#state !== 'jwks-armed') throw Error('JWKS rejection requires a new native TLS connection');
  }
}

export async function finishNativeAuthorizationSubmission(submission, captured, held) {
  // Attach immediately; the capture deadline can expire while form submission waits.
  captured.catch(() => {});
  try { await submission; }
  catch { if (!held()) throw Error('provider form did not issue an original callback'); }
  await captured;
}
