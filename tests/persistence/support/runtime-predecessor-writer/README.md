# Accepted Runtime writer fixture

Compile this fixture against the independently verified accepted 0.0.3 source.
Do not compile it against current workspace crates to establish historical compatibility.

Replace `@SOURCE_ROOT@` in `Cargo.toml.in` with the verified extraction path.
Resolve dependencies from a copy of the accepted lock.
Compare each resolved dependency with that lock before compilation.
Record source hashes, the resolved lock, compiler identity, executable hash, commands, and results.

Run the executable with `sqlite|redb DIRECTORY`.
The directory must exist and contain no `source`, `before.rombk`, or `summary.json`.
The writer runs three public Runtime commands with a versioned custom field codec.
The action produces one stored effect. The writer does not call a delivery provider.

The current maintained trial uses `ROM_RUNTIME_PREDECESSOR_EVIDENCE` and its explicit ignored test.
Default workspace success does not establish this historical-input gate.
Preserve all failed inputs and logs. Use a fresh directory for each repeat.
