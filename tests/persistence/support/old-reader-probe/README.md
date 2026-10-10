# Previous-release native reader probe

This probe must compile against separately verified accepted 0.0.3 source.
Accepted revision: `584c01b614127b3f62799f1a26a1cdf3f734c3dc`.
Do not build it against the current workspace or infer binary identity from its filename.

Copy this probe into a new evidence directory. Replace `@SOURCE_ROOT@` in its manifest template with the verified extraction's absolute path.
Record extraction hashes, archived commit/tree identity, the generated probe manifest, and source hashes before compilation.
Generate and retain a dedicated lock using the admitted previous-release dependency graph.
Review its dependency versions against the old release lock. Build with that frozen lock and a separate allocated target.
Record the compiler, exact command, probe lock hash, and executable SHA-256.

Run the probe first against the populated format-8 positive-control copy for each adapter.
Run it next against the format-9 copy for each adapter with expected outcome `reject`.
A missing path fails before adapter open. Only `Error::Unsupported` satisfies rejection.
Record main database bytes and applicable SQLite WAL bytes before and after the rejection command.
A positive open can initialize SQLite runtime sidecars; keep positive-control evidence separate from byte-preserving rejection evidence.
No historical source, live database, release artifact, or existing evidence may be modified.

The current release tests create retained populated predecessor and upgraded database paths.
Use those recorded paths without searching unrelated temporary directories.
This probe alone does not prove archive reader rejection or complete upgrade correctness.
