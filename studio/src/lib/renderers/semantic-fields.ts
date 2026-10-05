import type { CodecIdentity, WireValue } from "../client/types.ts";

export const SEMANTIC_KINDS = [
  "date",
  "time",
  "datetime",
  "color",
  "email",
  "url",
  "multiline",
  "json-document",
  "decimal",
  "unit-value",
] as const;
export type SemanticKind = (typeof SEMANTIC_KINDS)[number];
export function semanticKind(codec?: CodecIdentity): SemanticKind | undefined {
  if (codec?.version !== 1) return undefined;
  return SEMANTIC_KINDS.find((kind) => codec.name === `rom.${kind}`);
}
const bytes = (value: string) => new TextEncoder().encode(value).length;
function text(value: WireValue): string {
  if (typeof value !== "string")
    throw Error("This field requires a text representation.");
  return value;
}
function dateParts(value: string): number[] {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) throw Error("Use YYYY-MM-DD.");
  const [year, month, day] = match.slice(1).map(Number);
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const days = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  if (
    year < 1 ||
    year > 9999 ||
    month < 1 ||
    month > 12 ||
    day < 1 ||
    day > days[month - 1]
  )
    throw Error("Enter a valid Gregorian date, year 0001–9999.");
  return [year, month, day];
}
function timeParts(value: string): { parts: number[]; fraction: string } {
  const match = /^(\d{2}):(\d{2}):(\d{2})(?:\.(\d{1,9}))?$/.exec(value);
  if (!match) throw Error("Use HH:MM:SS with optional nanoseconds.");
  const parts = match.slice(1, 4).map(Number);
  if (parts[0] > 23 || parts[1] > 59 || parts[2] > 59)
    throw Error("Enter a valid time; leap seconds are not supported.");
  return { parts, fraction: match[4] ?? "" };
}
function decimal(value: string): string {
  if (bytes(value) > 1024 || !/^-?\d+(?:\.\d+)?$/.test(value))
    throw Error("Enter an exact decimal without an exponent.");
  const negative = value.startsWith("-");
  const [whole, fraction = ""] = (negative ? value.slice(1) : value).split(".");
  const integer = whole.replace(/^0+/, "") || "0";
  const tail = fraction.replace(/0+$/, "");
  return `${negative && (integer !== "0" || tail) ? "-" : ""}${integer}${tail ? `.${tail}` : ""}`;
}
function jsonDocument(value: string): string {
  if (bytes(value) > 1048576) throw Error("Document limit is 1 MiB.");
  let depth = 0,
    quoted = false,
    escaped = false;
  for (const character of value) {
    if (quoted) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === '"') quoted = false;
    } else if (character === '"') quoted = true;
    else if (character === "{" || character === "[") {
      if (++depth > 128) throw Error("Document nesting limit reached.");
    } else if (character === "}" || character === "]") depth--;
  }
  try {
    JSON.parse(value);
  } catch {
    throw Error("Enter valid JSON.");
  }
  // Parsing checks syntax only. Never emit or stringify its lossy numeric result.
  return value;
}
export function normalizeSemantic(
  kind: SemanticKind,
  value: WireValue,
): WireValue {
  if (kind === "unit-value") {
    if (
      !value ||
      typeof value !== "object" ||
      Array.isArray(value) ||
      Object.keys(value).length !== 2 ||
      typeof value.value !== "string" ||
      typeof value.unit !== "string" ||
      !/^[A-Za-z0-9_\-/.%^]{1,64}$/.test(value.unit)
    )
      throw Error("Enter an exact magnitude and an explicit unit token.");
    return { value: decimal(value.value), unit: value.unit };
  }
  const input = text(value);
  switch (kind) {
    case "date":
      dateParts(input);
      return input;
    case "time": {
      const parsed = timeParts(input),
        fraction = parsed.fraction.replace(/0+$/, "");
      return `${input.slice(0, 8)}${fraction ? `.${fraction}` : ""}`;
    }
    case "datetime": {
      const match =
        /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?)(Z|[+-]\d{2}:\d{2})$/.exec(
          input,
        );
      if (!match || match[3] === "-00:00")
        throw Error("Use an RFC3339 timestamp with a known timezone offset.");
      const [year, month, day] = dateParts(match[1]),
        time = timeParts(match[2]);
      const offset =
        match[3] === "Z"
          ? 0
          : Number(match[3].slice(1, 3)) * 60 + Number(match[3].slice(4));
      if (offset > 1439 || (match[3] !== "Z" && Number(match[3].slice(4)) > 59))
        throw Error("Enter a valid timezone offset.");
      const instant = new Date(0);
      instant.setUTCFullYear(year, month - 1, day);
      instant.setUTCHours(
        ...([time.parts[0], time.parts[1], time.parts[2], 0] as [
          number,
          number,
          number,
          number,
        ]),
      );
      instant.setUTCMinutes(
        instant.getUTCMinutes() - offset * (match[3][0] === "-" ? -1 : 1),
      );
      if (instant.getUTCFullYear() < 1 || instant.getUTCFullYear() > 9999)
        throw Error("UTC date must remain within year 0001–9999.");
      return `${instant.toISOString().slice(0, 19)}.${time.fraction.padEnd(9, "0")}Z`;
    }
    case "color":
      if (!/^#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?$/.test(input))
        throw Error("Use #RRGGBB or #RRGGBBAA.");
      return input.toLowerCase();
    case "decimal":
      return decimal(input);
    case "email": {
      const parts = input.split("@"),
        [local = "", domain = ""] = parts;
      if (
        parts.length !== 2 ||
        bytes(input) > 254 ||
        bytes(local) > 64 ||
        bytes(domain) > 253 ||
        !local
          .split(".")
          .every((atom) => /^[A-Za-z0-9!#$%&'*+\-/=?^_`{|}~]+$/.test(atom)) ||
        !domain
          .split(".")
          .every(
            (label) =>
              label.length <= 63 &&
              /^[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?$/.test(label),
          )
      )
        throw Error("Enter an ASCII mailbox with a DNS domain.");
      return `${local}@${domain.toLowerCase()}`;
    }
    case "url": {
      if (
        bytes(input) > 8192 ||
        /[\p{White_Space}\u0000-\u001f\u007f-\u009f\\]/u.test(input) ||
        !input.includes("://")
      )
        throw Error("Enter an absolute HTTP or HTTPS URL without whitespace.");
      let parsed: URL;
      try {
        parsed = new URL(input);
      } catch {
        throw Error("Enter a valid URL.");
      }
      if (
        !["http:", "https:"].includes(parsed.protocol) ||
        !parsed.hostname ||
        parsed.username ||
        parsed.password
      )
        throw Error("URL scheme or credentials are not allowed.");
      return parsed.href;
    }
    case "multiline":
      if (bytes(input) > 1048576) throw Error("Text limit is 1 MiB.");
      return input;
    case "json-document":
      return jsonDocument(input);
  }
}
