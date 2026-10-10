import test from "node:test";
import assert from "node:assert/strict";
import { resolveSourceLink } from "../../src/lib/ui/source-link.ts";
const options = {
  base: "https://studio.example/resources/",
  internal: (path: string) => path.startsWith("/resources/"),
  externalOrigins: ["https://docs.example"],
};

test("normalizes approved internal and external links without fetching", () => {
  assert.deepEqual(resolveSourceLink("./item?view=compact#fields", options), {
    href: "https://studio.example/resources/item?view=compact#fields",
    external: false,
  });
  assert.deepEqual(
    resolveSourceLink("https://docs.example:443/guide", options),
    {
      href: "https://docs.example/guide",
      external: true,
    },
  );
  assert.equal(resolveSourceLink("/admin", options), null);
});

test("scheme and credentials checks precede any internal callback", () => {
  let invoked = false;
  const checked = {
    ...options,
    internal: () => {
      invoked = true;
      return true;
    },
  };
  for (const raw of [
    "blob:https://studio.example/object",
    "javascript:alert(1)",
    "data:text/html,hello",
    "https://user:password@studio.example/resources/a",
  ]) {
    assert.equal(resolveSourceLink(raw, checked), null);
  }
  assert.equal(invoked, false);
});

test("origin comparisons reject lookalike hosts and protocol-relative outsiders", () => {
  for (const raw of [
    "//other.example/path",
    "https://docs.example.evil.test/guide",
    "http://docs.example/guide",
    "https://docs.example:8443/guide",
  ]) {
    assert.equal(resolveSourceLink(raw, options), null);
  }
});

test("internal callback sees immutable normalized path and failures reject links", () => {
  let inspected = "";
  assert.equal(
    resolveSourceLink("/resources/../admin", {
      ...options,
      internal: (path) => {
        inspected = path;
        return false;
      },
    }),
    null,
  );
  assert.equal(inspected, "/admin");
  assert.equal(
    resolveSourceLink("/resources/a", {
      ...options,
      internal: () => {
        throw new Error("host validation");
      },
    }),
    null,
  );
});

test("raw input has a fixed UTF8 bound and rejects control characters", () => {
  assert.equal(
    resolveSourceLink("/resources/" + "a".repeat(8192), options),
    null,
  );
  assert.equal(
    resolveSourceLink("/resources/" + "ą".repeat(4096), options),
    null,
  );
  assert.equal(
    resolveSourceLink("https://docs.exa\nmple/guide", options),
    null,
  );
});

test("unsafe or ambiguous configuration fails explicitly", () => {
  for (const base of [
    "file:///tmp/",
    "https://user@studio.example/",
    "not a URL",
  ]) {
    assert.throws(
      () => resolveSourceLink("/resources/a", { ...options, base }),
      /base/i,
    );
  }
  for (const origin of [
    "https://docs.example/path",
    "https://user@docs.example",
    "blob:https://docs.example/id",
    "https://docs.example?x=1",
  ]) {
    assert.throws(
      () =>
        resolveSourceLink("/resources/a", {
          ...options,
          externalOrigins: [origin],
        }),
      /origin/i,
    );
  }
});

test("a JavaScript async internal validator cannot approve a blocked link", async () => {
  const result = resolveSourceLink("/blocked", {
    ...options,
    // @ts-expect-error JavaScript consumers can supply an invalid async callback.
    internal: async () => false,
  });
  assert.equal(result, null);
});

test("a rejected async validator is ignored without an unhandled rejection", async () => {
  assert.equal(
    resolveSourceLink("/blocked", {
      ...options,
      // @ts-expect-error An invalid JS callback must not approve or leak its rejection.
      internal: async () => {
        throw new Error("invalid async validator");
      },
    }),
    null,
  );
  await new Promise<void>((resolve) => setImmediate(resolve));
});
