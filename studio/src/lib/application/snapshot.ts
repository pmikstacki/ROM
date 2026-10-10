import { parseWire, stringifyWire } from "../client/codec.ts";
import type {
  Operation,
  ProjectedView,
  QuerySpec,
  WireValue,
} from "../client/types.ts";
import type { ApplicationState } from "./controller.ts";
import type { ApplicationRecoverySnapshot } from "./session-types.ts";
import type {
  EditorPersistenceState,
  EditorSnapshot,
} from "./editor-drafts.ts";

/** Only explicit wire trees traverse the codec. Typed application counters retain number types. */
function wire<T>(value: T, maxBytes: number): T {
  return parseWire(
    stringifyWire(value as unknown as WireValue, maxBytes),
    maxBytes,
  ) as T;
}
function view(
  value: ProjectedView | null,
  maxBytes: number,
): ProjectedView | null {
  return value === null
    ? null
    : {
        key: { ...value.key },
        revision: value.revision,
        value: wire(value.value, maxBytes),
      };
}
export function copyEditorSnapshot(
  value: EditorSnapshot,
  maxBytes: number,
): EditorSnapshot {
  // Editor snapshots have an explicit wire persistence contract, including custom editor state.
  return { ...wire(value, maxBytes), baseRevision: value.baseRevision };
}
export function copyEditorState(
  value: EditorPersistenceState,
  maxBytes: number,
): EditorPersistenceState {
  return {
    status: value.status,
    target: value.target ? { ...value.target } : null,
    snapshot: value.snapshot
      ? copyEditorSnapshot(value.snapshot, maxBytes)
      : null,
  };
}
function query(value: QuerySpec, maxBytes: number): QuerySpec {
  const copied = structuredClone(value);
  // Container copies preserve numeric lexical metadata on the value member.
  if (value.filters) copied.filters = wire(value.filters, maxBytes);
  if (value.comparisons) copied.comparisons = wire(value.comparisons, maxBytes);
  return copied;
}
function recovery(
  value: ApplicationRecoverySnapshot,
  maxBytes: number,
): ApplicationRecoverySnapshot {
  return {
    target: { ...value.target },
    state: {
      ...value.state,
      draft: value.state.draft
        ? wire<Operation>(value.state.draft, maxBytes)
        : null,
      result: view(value.state.result, maxBytes),
      error: value.state.error ? { ...value.state.error } : null,
    },
  };
}
export function copyApplicationState(
  value: ApplicationState,
  maxBytes: number,
): ApplicationState {
  const copied = {
    ...value,
    descriptors: structuredClone(value.descriptors),
    rows: value.rows.map((row) => view(row, maxBytes)!),
    selected: view(value.selected, maxBytes),
    query: query(value.query, maxBytes),
    work: wire(value.work, maxBytes),
  };
  if (value.session) copied.session = { ...value.session };
  if (value.editor) copied.editor = copyEditorState(value.editor, maxBytes);
  if (value.creation)
    copied.creation = {
      busy: value.creation.busy,
      editor: {
        status: value.creation.editor.status,
        snapshot: value.creation.editor.snapshot
          ? {
              id: value.creation.editor.snapshot.id,
              form: copyEditorSnapshot(
                value.creation.editor.snapshot.form,
                maxBytes,
              ),
            }
          : null,
      },
      recovery: value.creation.recovery
        ? recovery(value.creation.recovery, maxBytes)
        : null,
    };
  if (value.recovery) copied.recovery = recovery(value.recovery, maxBytes);
  if (value.pending)
    copied.pending = {
      ...value.pending,
      request: wire(value.pending.request, maxBytes),
      ...(value.pending.result
        ? { result: view(value.pending.result, maxBytes)! }
        : {}),
    };
  return copied;
}
