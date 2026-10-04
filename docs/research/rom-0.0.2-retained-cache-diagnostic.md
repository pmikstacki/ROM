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

## Retained-image carrier inventory

An independently reviewed read-only diagnostic inspected the selected image
after a fresh no-writer and free-space check. It completed without a timeout.
The outer result is
`/root/ipi/research/disk-coordination-2026-10-04/ROM-carrier-diagnostic-run.json`.
Its SHA-256 is
`c8bdad3e60bcae7688c163bbadf1de012577b4fe8a83661218d98432b1b14bfc`.

The physical traversal visited 6,775 entries. It found 19 opaque markers.
Every marker was on a directory and had the value `y`. It found zero
character-device whiteouts. The 19 carriers matched the count from the
separate xattr-name scan. The traversal metadata and xattr digest was
`a415c835c25a301813ea5a95c02a5d2c2261f25e2a1a91c2ccdd399bf0845a05`.
This digest does not include file contents.

The full image SHA-256 before and after was
`3c848435186404e8cd7f7d0d403ed7ea64609b105c149dd76fb805ba0b7968db`.
The guardian reported no remaining owned child, loop association, or cleanup
error. The result retains `diagnosticOnly=true` and `compatible=false`.

This resolves the carrier and whiteout inventory for this image. It does not
verify the 547 origin handles in a merged lower view. Test that exact overlay
composition before admitting the image as a build cache.

## Merged-lower admission boundary

An independent review examined the Linux 6.6.94 OverlayFS implementation.
The 547 stored `trusted.overlay.origin` values are not decoded as origin
handles merely because the former upper becomes a lower layer. The proposed
two-lower probe can test merged names, types, contents, opaque directories,
and writes into a fresh upper layer. It cannot establish that each old origin
handle remains valid. Linux processes whiteouts and opaque directories during
lower lookup. Origin verification applies to a current upper entry. See the
[6.6.94 lookup implementation](https://raw.githubusercontent.com/gregkh/linux/v6.6.94/fs/overlayfs/namei.c)
and [OverlayFS documentation](https://raw.githubusercontent.com/gregkh/linux/v6.6.94/Documentation/filesystems/overlayfs.rst).

Do not run the exact two-lower admission probe while the original lower may
change. Its host filesystem is writable and `container@rom-dev.service` is
active. A read-only bind would stop probe writes. It would not stop the
container or another host writer. Linux documents underlying changes during
an OverlayFS mount as undefined behavior. Prove that the source is quiet or
use an immutable snapshot before treating this test as admission evidence.
A copied lower can provide a narrower merge experiment. Its changed inode
and filesystem identity do not establish exact-source provenance.

Once the source is stable, use the retained target upper and original target
as two read-only lowers. Use a small fresh tmpfs upper and work directory.
Check the complete merged tree against the raw layers. Check representative
origin-carrier paths before and after copy-up. Check full input digests,
effective mount options, and clean mount and loop teardown. The eight-gate
clean-source producer remains the release test.
