# Retained cache name diagnostic

Date: 2026-10-04 UTC. This is a read-only diagnostic, not a cache admission
or a ROM 0.0.2 release result.

The prepared v3 observer examined the retained failed producer image at
`/var/tmp/rom-bounded-producer-oDMHqt/build.ext4`. A fresh sibling package
repinned the changed skills process helper. An independent reviewer found no
other executable package change. All 20 synthetic diagnostic tests passed.

The bounded run started at 17:49:06 UTC and ended at 17:49:45 UTC. Its outer
controller returned code 0 without a timeout. The image SHA-256 before and
after was
`3c848435186404e8cd7f7d0d403ed7ea64609b105c149dd76fb805ba0b7968db`.
The guardian reported no remaining owned child, loop association, or cleanup
error. The complete captured `getfattr` output was 117,979 bytes with SHA-256
`6b559eb472c3872a225db6fb8e35242cf0ddbd984498f36babe256c6af31263b`.

| Attribute name | Occurrences | Recorded value class |
| --- | ---: | --- |
| `trusted.overlay.impure` | 101 | One byte; example `0x79` |
| `trusted.overlay.opaque` | 19 | One byte; example `0x79` |
| `trusted.overlay.origin` | 547 | 29-byte version 0 handle header; nonzero UUID |
| `trusted.overlay.uuid` | 1 | 16 bytes |

The profile counted 668 occurrences. It reported complete name coverage and
no omitted names, classes, or examples. The profile did not identify every
attribute carrier type. Its value classes do not prove that origin handles
resolve in a new lower-layer role. The run explicitly reported
`diagnosticOnly=true` and `compatible=false`.

The retained result is
`/root/ipi/research/disk-coordination-2026-10-04/ROM-name-diagnostic-v3-run.json`.
Its bounded outer stdout SHA-256 is
`efd77a23d767981b7945d8531438d267fc525f63ba6dca77c0d9ae37ba8c54c7`.
The reviewed package manifest SHA-256 is
`ccbae29eb7df6adccc901d244a2aaec1a7298ea41b58ec09f3022fc3902ed76d`.

Before cache reuse, verify carrier types and the effect of opaque directories,
whiteouts, and origin metadata in the exact proposed lower-layer order. Keep
both lower inputs immutable. Retain the complete eight-gate producer as the
release acceptance test.

## Synthetic role transition

The bounded probe ran on kernel 6.6.94 at 18:04:30 UTC. It used an 8 MiB
private tmpfs and did not mount the retained image. The first overlay created
an opaque directory and two character-device whiteouts. The probe then mounted
the old upper layer above the original lower layer as two read-only lowers.
The merged view kept the intended additions and deletions. A later write and
delete went to a fresh upper layer. The lower-tree snapshot was unchanged.

The controller returned code 0 without a timeout. It found no host mount or
entries at its former mountpoint. The result is
`/root/ipi/research/disk-coordination-2026-10-04/ROM-overlay-role-transition-result.json`.
Its lower snapshot SHA-256 was
`e07fd80d89228860f6b8651672661b9751b4364b935a9026ab6a527830a64ae8`.

This test proves the selected behavior for a small synthetic filesystem. It
does not prove that the retained image has compatible carrier types, whiteout
inventory, or origin handles. Inspect those properties before cache admission.
The complete producer and its eight gates remain required.
