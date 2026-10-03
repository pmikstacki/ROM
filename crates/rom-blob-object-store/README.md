# rom-blob-object-store

`Adapter::trusted_folder(root, max_bytes)` and
`Adapter::s3(S3Config { .. }, max_bytes)` implement the same `rom_blob::BlobStore`
port. The default runtime contract uses atomic create-only publication; there is
no overwrite or head-then-put emulation. Unsupported provider conditions fail.
Reads compare metadata and streamed bytes with both caller and adapter limits.
Deletion of a missing object succeeds. Transport uncertainty during a mutation returns `Unknown`.

Folder roots and descendants must be exclusively trusted. The underlying library
follows symlinks. Path validation is not a filesystem sandbox. The adapter enables
file/directory fsync. Tests exercise normal reopen, not machine/power failure.

S3 configuration is explicit and host-owned. HTTPS is mandatory except for the
deliberate numeric-loopback test policy. No userinfo, fragment or query is accepted
in the endpoint. Credentials never appear in adapter Debug/error output. Requests
use a three-second timeout, one-second connect timeout and zero automatic retries.
The endpoint must implement conditional create (If-None-Match). The test suite
certifies compatibility; an S3 label alone does not establish compatibility. Multipart, signed
URLs and provider version retrieval are not exposed.

`cargo test -p rom-blob-object-store --locked` runs folder, metadata restart and
input-policy tests. Real S3 tests are explicitly ignored by default. Run:

```sh
MINIO_BIN=/path/to/minio ./crates/rom-blob-object-store/verify-s3
```

The script refuses occupied ports. It starts disposable loopback MinIO on ports
19180/19181 with fixture-only credentials, runs all adapter cases and stops the server.
The S3 fault fixture forwards a real signed create. After MinIO accepts it, the
fixture drops the response. The adapter must report Unknown and independent GET must recover the
complete content. This proves the selected MinIO profile, not AWS/TLS interoperability.

The package is MIT licensed. Its private Apache Arrow Object Store dependency is
Apache-2.0; the upstream `NOTICE` and `OBJECT_STORE_LICENSE` are retained alongside
this package. Provider SDK types do not enter the core or public blob port.
