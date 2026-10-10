// A completed owned relay is a setup failure, even when its exit status is zero.
export function requirePendingRelay(result){if(result!==undefined)throw Error('provider relay exited before readiness');}
