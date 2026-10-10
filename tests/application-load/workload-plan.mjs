import { createHash } from "node:crypto";
const targets = Object.freeze({
  read: 9040,
  mutation: 1500,
  replay: 820,
  blob: 640,
});
const digest = createHash("sha256")
  .update(Buffer.alloc(65536, 0x61))
  .digest("hex");
function mutation(index) {
  return {
    kind: "load-records",
    id: `load-${String(index).padStart(5, "0")}`,
    expected: 1,
    idempotency: `load-touch-${index}`,
    operation: { type: "action", input: { name: "touch", input: index < 500 } },
  };
}
function requests(category, index) {
  if (category === "mutation" || category === "replay")
    return [
      {
        method: "POST",
        path: "/rom-studio/api/invoke",
        body: mutation(index),
        measurement: category === "mutation" ? "commit" : "replay",
      },
    ];
  if (category === "blob") {
    const id = `load-attachment-${index}`;
    return [
      {
        method: "POST",
        path: "/rom-studio/blobs/reserve",
        body: {
          id,
          store: "attachments",
          digest,
          bytes: 65536,
          idempotency: `load-reserve-${index}`,
        },
        measurement: "reserve",
      },
      {
        method: "POST",
        path: `/rom-studio/blobs/upload?id=${id}`,
        binary_bytes: 65536,
        measurement: "upload",
      },
      {
        method: "GET",
        path: `/rom-studio/blobs/attachment/${id}`,
        expected_bytes: 65536,
        measurement: "download",
      },
    ];
  }
  if (index % 4 === 0)
    return [
      {
        method: "POST",
        path: "/rom-studio/api/query",
        body: {
          kind: "load-records",
          query: { filters: [{ field: "open", value: true }], limit: 50 },
        },
        measurement: "query",
      },
    ];
  return [
    {
      method: "POST",
      path: "/rom-studio/api/read",
      body: {
        kind: "load-records",
        id: `load-${String(index % 10000).padStart(5, "0")}`,
      },
      measurement: "read",
    },
  ];
}
export function* groups(options) {
  if (options !== undefined) throw Error("closed workload");
  const counts = { read: 0, mutation: 0, replay: 0, blob: 0 };
  for (let index = 0; index < 12000; index++) {
    let selected,
      largest = -Infinity;
    for (const category of Object.keys(targets)) {
      if (counts[category] === targets[category]) continue;
      const deficit =
        (targets[category] * (index + 1)) / 12000 - counts[category];
      if (deficit > largest) {
        largest = deficit;
        selected = category;
      }
    }
    const sequence = counts[selected]++;
    yield Object.freeze({
      index,
      planned: index * 10,
      measured: index >= 3000,
      category: selected,
      requests: requests(selected, sequence),
    });
  }
}
