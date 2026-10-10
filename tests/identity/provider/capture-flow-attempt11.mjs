// Authoring facade. Exact prior attempted source is retained in private run evidence.
import { captureOriginalFlow } from './original-flow-capture.mjs';
await captureOriginalFlow({ privateDirectory: process.argv[2], attemptName: 'attempt11' });
