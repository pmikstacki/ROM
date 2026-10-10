// Safe inspect projections only. Raw Config.Env and administrative values never enter evidence.
export function requireContainerIdentity(expected, actual) {
  const digest = /^[a-f0-9]{64}$/;
  const network = expected.network_container ? `container:${expected.network_container}` : 'none';
  if (!digest.test(expected.id) || !digest.test(expected.image) || !/^[a-f0-9]{32}$/.test(expected.nonce) ||
      !/^\/rom-identity-[a-f0-9]{32}$/.test(expected.cgroup) ||
      (expected.network_container !== undefined && !digest.test(expected.network_container)) ||
      actual?.id !== expected.id || actual.image !== expected.image || actual.label !== expected.nonce ||
      actual.privileged !== false || actual.network !== network || actual.cgroup !== expected.cgroup ||
      !Number.isSafeInteger(actual.pid) || actual.pid <= 1 ||
      !Number.isSafeInteger(actual.memory) || actual.memory <= 0 || actual.memory > 1610612736 ||
      actual.cpu_period !== 100000 || !Number.isSafeInteger(actual.cpu_quota) || actual.cpu_quota <= 0 || actual.cpu_quota > 150000 ||
      !Number.isSafeInteger(actual.pids) || actual.pids <= 0 || actual.pids > 128) throw Error('owned container identity mismatch');
  return actual;
}

export function safeContainerProjection(value) {
  const host = value.HostConfig ?? {};
  return { id: value.Id, image: value.Image.replace(/^sha256:/, ''), label: value.Config?.Labels?.['org.rom.identity-fixture'],
    pid: value.State?.Pid, privileged: host.Privileged, network: host.NetworkMode, cgroup: host.CgroupParent,
    memory: host.Memory, cpu_period: host.CpuPeriod, cpu_quota: host.CpuQuota, pids: host.PidsLimit };
}
