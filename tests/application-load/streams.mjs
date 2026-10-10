import { serverTerminalCategory } from "./server-terminal-category.mjs";
const origin = "https://127.0.0.1:44389";
// One selected-row stream per reader; slow mode keeps the actual response unread.
export async function openLoadStreams(
  session,
  { normal = 4, slow = 1 } = {},
  acquire = fetch,
) {
  if (
    !Number.isSafeInteger(normal) ||
    !Number.isSafeInteger(slow) ||
    normal < 1 ||
    slow < 0 ||
    slow > 1 ||
    normal + slow > 32 ||
    typeof acquire !== "function"
  )
    throw Error("finite load stream profile");
  const controls = [],
    closed = [];
  let bytes = 0,
    data = 0,
    errors = 0,
    refused = 0;
  const errorClasses = {};
  const countError = reason => { errorClasses[reason] = Math.min(4194304, (errorClasses[reason] ?? 0) + 1); };
  try {
    for (let index = 0; index < normal + slow; index++) {
      const controller = new AbortController(),
        response = await acquire(origin + "/rom-studio/api/live", {
          method: "POST",
          headers: {
            cookie: session.cookie,
            origin,
            "x-rom-csrf": session.csrf,
            "content-type": "application/json",
          },
          body: JSON.stringify({
            kind: "load-records",
            query: {
              filters: [
                {
                  field: "title",
                  value: `Synthetic load row ${String(index).padStart(5, "0")}`,
                },
              ],
              limit: 1,
            },
          }),
          redirect: "error",
          signal: AbortSignal.any([
            controller.signal,
            AbortSignal.timeout(150000),
          ]),
        });
      controls.push({ controller, response });
      if (
        response.status !== 200 ||
        !response.headers
          .get("content-type")
          ?.startsWith("text/event-stream") ||
        !response.body
      ) {
        refused++;
        throw Error("actual load stream rejected");
      }
      if (index >= normal) continue;
      const loop = (async () => {
        const reader = response.body.getReader(),
          decoder = new TextDecoder();
        controls.at(-1).reader = reader;
        let pending = "";
        let failureStage = "reader-exception";
        try {
          while (true) {
            failureStage = "reader-exception";
            const chunk = await reader.read();
            if (chunk.done) {
              if (!controller.signal.aborted) {
                failureStage = "early-eof";
                throw Error("load stream ended before owned cancellation");
              }
              break;
            }
            bytes += chunk.value.length;
            if (bytes > 4 * 1024 ** 2) { failureStage = "byte-budget"; throw Error("finite stream byte budget"); }
            pending += decoder.decode(chunk.value, { stream: true });
            if (pending.length > 65536) { failureStage = "frame-budget"; throw Error("bounded stream frame"); }
            let boundary;
            while ((boundary = pending.indexOf("\n\n")) !== -1) {
              const frame = pending.slice(0, boundary);
              pending = pending.slice(boundary + 2);
              if (frame.startsWith(":")) continue;
              const type = /^event: ?([^\n]+)$/m.exec(frame)?.[1];
              if (type === "error") {
                const code = frame.split("\n").filter(line => line.startsWith("data:")).map(line => line.slice(5).trimStart()).join("\n");
                const category = serverTerminalCategory(code);
                countError("server-" + category);
                errors++;
                continue;
              }
              if (type !== "data") { failureStage = "event-type"; throw Error("closed stream event type"); }
              failureStage = "data-shape";
              const value = JSON.parse(
                frame
                  .split("\n")
                  .filter((line) => line.startsWith("data:"))
                  .map((line) => line.slice(5).trimStart())
                  .join("\n"),
              );
              if (
                !Array.isArray(value) ||
                value.length > 1 ||
                value.some(
                  (row) =>
                    row?.key?.kind !== "load-records" ||
                    typeof row.key.id !== "string",
                )
              )
                throw Error("actual stream Resource response");
              data++;
            }
          }
        } catch {
          if (!controller.signal.aborted) { errors++; countError(failureStage); }
        } finally {
          await reader.cancel().catch(() => {});
          reader.releaseLock();
        }
      })();
      closed.push(loop);
    }
  } catch (error) {
    for (const control of controls) {
      control.controller.abort();
      if (control.reader) {
        try {
          await control.reader.cancel();
        } catch {}
      }
      if (!control.response.body?.locked)
        await control.response.body?.cancel().catch(() => {});
    }
    await Promise.all(closed);
    throw error;
  }
  return {
    counts: () => ({
      normal,
      slow,
      bytes,
      data_events: data,
      error_events: errors,
      error_classes: { ...errorClasses },
      refused,
      maximum_bytes: 4 * 1024 ** 2,
    }),
    async close() {
      for (const control of controls) {
        control.controller.abort();
        if (control.reader) {
          try {
            await control.reader.cancel();
          } catch {}
        }
        if (!control.response.body?.locked)
          await control.response.body?.cancel().catch(() => {});
      }
      await Promise.all(closed);
      return { readers_drained: true, ...this.counts() };
    },
  };
}
