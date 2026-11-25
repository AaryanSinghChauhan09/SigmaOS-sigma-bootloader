# SigmaOS Bootloader (sigma-bootloader)

Multiboot2 boot header and tag generator embedded in the kernel image,
plus a Multiboot1 header for loaders such as QEMU `-kernel`.

## Capabilities

- `Mb2Header::sigmaos_default()` — builds header requesting memory map,
  framebuffer, cmdline, and boot loader name
- Framebuffer request tag (any resolution, 32bpp)
- Kernel address tag + module tags (initramfs etc.)
- Checksum verification, 8-byte tag alignment
- `Mb1Header` — magic/flags/checksum serialization for Multiboot1
- `to_bytes()` serialization for linking into the kernel ELF

## Interconnectivity

| Repo | Relationship |
|------|-------------|
| `sigmaos-sigma-kernel` | Kernel parses the MB2 info this requests |
| `sigmaos-initramfs` | Produces module payloads referenced by module tags |
| `sigmaos-ISO` | ISO build links the header into the boot image |

## Status

Real implementation (was stub). 10 tests passing, `#![no_std]`,
`#![forbid(unsafe_code)]`, GPL-3.0.
