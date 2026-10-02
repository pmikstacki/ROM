# rom-blob-object-store

`Adapter::trusted_folder(root, max_bytes)` and
`Adapter::s3(S3Config { .. }, max_bytes)` implement the same `rom_blob::BlobStore`
port. The default runtime contract uses atomic create-only publication; there is
no overwrite or head-then-put emulation. Unsupported provider conditions fail.
Reads check metadata and streamed bytes against both caller and adapter limits;
missing deletes succeed, and mutating transport uncertainty returns `Unknown`.

Folder roots and descendants must be exclusively trusted. The underlying library
follows symlinks; path validation is not a filesystem sandbox. The adapter enables
file/directory fsync. Tests exercise normal reopen, not machine/power failure.

S3 configuration is explicit and host-owned. HTTPS is mandatory except for the
deliberate numeric-loopback test policy. No userinfo, fragment or query is accepted
in the endpoint. Credentials never appear in adapter Debug/error output. Requests
use a three-second timeout, one-second connect timeout and zero automatic retries.
The endpoint must implement conditional create (If-None-Match); compatibility is
certified by running the suite, not inferred from an S3 label. Multipart, signed
URLs and provider version retrieval are not exposed.

`cargo test -p rom-blob-object-store --locked` runs folder, metadata restart and
input-policy tests. Real S3 tests are explicitly ignored by default. Run:

```sh
MINIO_BIN=/path/to/minio ./crates/rom-blob-object-store/verify-s3
```

The script starts disposable loopback MinIO on ports 19180/19181 with fixture-only
credentials, refuses occupied ports, runs all adapter cases and stops the server.
The S3 fault fixture forwards a real signed create, sees MinIO accept it, then drops
the response. The adapter must report Unknown and independent GET must recover the
complete content. This proves the selected MinIO profile, not AWS/TLS interoperability.

The package is MIT licensed. Its private Apache Arrow Object Store dependency is
Apache-2.0; the upstream `NOTICE` and `OBJECT_STORE_LICENSE` are retained alongside
this package. Provider SDK types do not enter the core or public blob port.
