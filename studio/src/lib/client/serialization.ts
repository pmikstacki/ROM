import type { WireObject, WireValue } from "./types.ts";

export const DEFAULT_BYTES = 1024 * 1024;
const encoder = new TextEncoder();
const numberToken = /-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/y;
const floatTokens = new WeakMap<object, Map<string, string>>();

/** Retained lexical category applies only to the original parsed container. */
export function floatingMember(owner: object, key: string): boolean {
  const token = floatTokens.get(owner)?.get(key);
  const current = Object.getOwnPropertyDescriptor(owner, key)?.value;
  return (
    token !== undefined &&
    typeof current === "number" &&
    Object.is(Number(token), current)
  );
}
function unicode(value: string): string {
  for (let index = 0; index < value.length; index++) {
    const code = value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(++index);
      if (!(next >= 0xdc00 && next <= 0xdfff))
        throw Error("invalid wire Unicode");
    } else if (code >= 0xdc00 && code <= 0xdfff)
      throw Error("invalid wire Unicode");
  }
  return value;
}

function limit(value: number, label: string): void {
  if (!Number.isSafeInteger(value) || value <= 0)
    throw Error(`invalid ${label} limit`);
}

/** Strict bounded JSON with exact integer tokens and null-prototype maps. */
export function parseWire(
  text: string,
  maxBytes = DEFAULT_BYTES,
  maxDepth = 64,
): WireValue {
  limit(maxBytes, "byte");
  limit(maxDepth, "depth");
  if (text.length > maxBytes || encoder.encode(text).byteLength > maxBytes)
    throw Error("wire byte limit");
  let position = 0;
  const whitespace = () => {
    while (/^[\t\n\r ]$/.test(text[position] ?? "")) position++;
  };
  const invalid = (): never => {
    throw Error(`invalid wire JSON at ${position}`);
  };
  function member(container: object, key: string, depth: number): WireValue {
    const start = position;
    const result = value(depth);
    if (typeof result === "number") {
      const token = text.slice(start, position).trim();
      if (/[.eE]/.test(token)) {
        let tokens = floatTokens.get(container);
        if (!tokens) {
          tokens = new Map();
          floatTokens.set(container, tokens);
        }
        tokens.set(key, token);
      }
    }
    return result;
  }
  function string(): string {
    if (text[position] !== '"') return invalid();
    const start = position++;
    while (position < text.length) {
      const character = text[position++];
      if (character === "\\") position++;
      else if (character === '"')
        return unicode(JSON.parse(text.slice(start, position)));
    }
    return invalid();
  }
  function value(depth: number): WireValue {
    if (depth > maxDepth) throw Error("wire depth limit");
    whitespace();
    const character = text[position];
    if (character === '"') return string();
    if (character === "{") {
      position++;
      const object: WireObject = Object.create(null);
      whitespace();
      if (text[position] === "}") {
        position++;
        return object;
      }
      while (true) {
        whitespace();
        const key = string();
        if (Object.hasOwn(object, key)) throw Error("duplicate wire key");
        whitespace();
        if (text[position++] !== ":") return invalid();
        object[key] = member(object, key, depth + 1);
        whitespace();
        const next = text[position++];
        if (next === "}") return object;
        if (next !== ",") return invalid();
      }
    }
    if (character === "[") {
      position++;
      const array: WireValue[] = [];
      whitespace();
      if (text[position] === "]") {
        position++;
        return array;
      }
      while (true) {
        array.push(member(array, String(array.length), depth + 1));
        whitespace();
        const next = text[position++];
        if (next === "]") return array;
        if (next !== ",") return invalid();
      }
    }
    for (const [token, result] of [
      ["true", true],
      ["false", false],
      ["null", null],
    ] as const) {
      if (text.startsWith(token, position)) {
        position += token.length;
        return result;
      }
    }
    numberToken.lastIndex = position;
    const match = numberToken.exec(text);
    if (!match) return invalid();
    position = numberToken.lastIndex;
    const token = match[0];
    if (token === "-0") return -0;
    if (token.length > 256) throw Error("wire number limit");
    if (/^-?\d+$/.test(token)) {
      const integer = BigInt(token);
      return integer >= BigInt(Number.MIN_SAFE_INTEGER) &&
        integer <= BigInt(Number.MAX_SAFE_INTEGER)
        ? Number(integer)
        : integer;
    }
    const floating = Number(token);
    if (!Number.isFinite(floating)) throw Error("wire number must be finite");
    return floating;
  }
  const result = value(0);
  whitespace();
  if (position !== text.length) return invalid();
  return result;
}

/** Serialize integer tokens without converting bigint to strings or floating point. */
export function stringifyWire(
  value: WireValue,
  maxBytes = DEFAULT_BYTES,
): string {
  limit(maxBytes, "byte");
  const fragments: string[] = [];
  const ancestors = new Set<object>();
  let bytes = 0;
  const append = (fragment: string) => {
    bytes += encoder.encode(fragment).byteLength;
    if (bytes > maxBytes) throw Error("wire byte limit");
    fragments.push(fragment);
  };
  function write(current: WireValue, depth: number, token?: string): void {
    if (depth > 64) throw Error("wire depth limit");
    if (typeof current === "bigint") {
      append(current.toString());
      return;
    }
    if (
      current === null ||
      typeof current === "string" ||
      typeof current === "boolean"
    ) {
      append(
        JSON.stringify(
          typeof current === "string" ? unicode(current) : current,
        ),
      );
      return;
    }
    if (typeof current === "number") {
      if (!Number.isFinite(current)) throw Error("wire number must be finite");
      append(
        token && Object.is(Number(token), current)
          ? token
          : Object.is(current, -0)
            ? "-0.0"
            : JSON.stringify(current),
      );
      return;
    }
    if (typeof current !== "object") throw Error("unsupported wire value");
    if (ancestors.has(current)) throw Error("cyclic wire value");
    ancestors.add(current);
    if (Array.isArray(current)) {
      append("[");
      for (let index = 0; index < current.length; index++) {
        if (index) append(",");
        const member = Object.getOwnPropertyDescriptor(current, String(index));
        if (!member || !Object.hasOwn(member, "value"))
          throw Error("wire array getters or holes are unsupported");
        write(
          member.value,
          depth + 1,
          floatTokens.get(current)?.get(String(index)),
        );
      }
      append("]");
    } else {
      if (
        ![null, Object.prototype].includes(Object.getPrototypeOf(current)) ||
        Object.getOwnPropertySymbols(current).length
      )
        throw Error("unsupported wire object");
      append("{");
      const members = Object.getOwnPropertyDescriptors(current);
      let index = 0;
      for (const key of Object.keys(members)) {
        const member = members[key];
        if (!member.enumerable) continue;
        if (!Object.hasOwn(member, "value"))
          throw Error("wire getters are unsupported");
        if (index++) append(",");
        append(JSON.stringify(unicode(key)));
        append(":");
        write(member.value, depth + 1, floatTokens.get(current)?.get(key));
      }
      append("}");
    }
    ancestors.delete(current);
  }
  write(value, 0);
  return fragments.join("");
}
