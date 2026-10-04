// Conventional private inputs are rejected by source archives and frontend copies.
export function isPrivateSourcePath(path) {
  return path.split('/').some(part => /^\.env(?:\.|$)/.test(part) && part !== '.env.example') || /\.(key|pem)$/.test(path);
}
