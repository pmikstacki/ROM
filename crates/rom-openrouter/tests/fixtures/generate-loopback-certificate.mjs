// Test-only self-signed certificate. This file is not used by production transport.
import { generateKeyPairSync, sign, X509Certificate } from "node:crypto";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const encoded = (tag, ...parts) => {
  const body = Buffer.concat(parts);
  const length = body.length < 128
    ? Buffer.from([body.length])
    : body.length < 256
      ? Buffer.from([0x81, body.length])
      : Buffer.from([0x82, body.length >> 8, body.length & 255]);
  return Buffer.concat([Buffer.from([tag]), length, body]);
};
const sequence = (...parts) => encoded(0x30, ...parts);
const oid = (hex) => encoded(0x06, Buffer.from(hex, "hex"));
const algorithm = sequence(oid("2a864886f70d01010b"), encoded(0x05));
const commonName = sequence(encoded(0x31, sequence(
  oid("550403"), encoded(0x0c, Buffer.from("ROM loopback test")),
)));
const validity = sequence(
  encoded(0x18, Buffer.from("20260101000000Z")),
  encoded(0x18, Buffer.from("20360101000000Z")),
);
const { publicKey, privateKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
const san = sequence(oid("551d11"), encoded(0x04, sequence(encoded(0x87, Buffer.from([127, 0, 0, 1])))));
const constraints = sequence(oid("551d13"), encoded(0x01, Buffer.from([255])), encoded(0x04, sequence()));
const extendedUsage = sequence(oid("551d25"), encoded(0x04, sequence(oid("2b06010505070301"))));
const tbs = sequence(
  encoded(0xa0, encoded(0x02, Buffer.from([2]))),
  encoded(0x02, Buffer.from([1])), algorithm, commonName, validity, commonName,
  publicKey.export({ type: "spki", format: "der" }),
  encoded(0xa3, sequence(san, constraints, extendedUsage)),
);
const certificate = sequence(tbs, algorithm, encoded(0x03, Buffer.from([0]), sign("sha256", tbs, privateKey)));
const parsed = new X509Certificate(certificate);
if (!parsed.verify(publicKey) || parsed.checkIP("127.0.0.1") !== "127.0.0.1") throw new Error("invalid fixture certificate");
writeFileSync(fileURLToPath(new URL("loopback-certificate.der", import.meta.url)), certificate);
writeFileSync(fileURLToPath(new URL("loopback-key.der", import.meta.url)), privateKey.export({ type: "pkcs8", format: "der" }));
// Generated private material is exclusively a public test fixture, never a credential.
console.log(`Self-signed fixture verified; SHA-256 fingerprint ${parsed.fingerprint256}`);
