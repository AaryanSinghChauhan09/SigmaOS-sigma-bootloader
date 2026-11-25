//! SigmaOS bootloader — Multiboot2 boot header and tag generation.
//!
//! Produces the MB2 header embedded in the kernel image: magic, arch,
//! header length, checksum, and header tags requesting framebuffer,
//! memory map, and module loading.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

/// Multiboot2 boot header magic.
pub const MB2_BOOT_MAGIC: u32 = 0x1BAD_B002;

/// Architecture: i386 (protected/long mode via our own entry).
pub const ARCH_I386: u32 = 0;

/// Header tag types.
pub mod tag_type {
    pub const END: u16 = 0;
    pub const INFORMATION_REQUEST: u16 = 1;
    pub const ADDRESS: u16 = 2;
    pub const FLAGS: u16 = 3;
    pub const FRAMEBUFFER: u16 = 5;
    pub const MODULE: u16 = 7;
    pub const RANDOM: u16 = 6;
}

/// Information request tag flag bits (info tag types we want).
pub mod info_flags {
    pub const MEM_MAP: u32 = 1 << 6;
    pub const FRAMEBUFFER: u32 = 1 << 5;
    pub const CMDLINE: u32 = 1 << 2;
    pub const BOOT_LOADER_NAME: u32 = 1 << 1;
    pub const MEMORY_MAP: u32 = MEM_MAP;
}

/// Errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootError {
    /// Header layout invalid.
    InvalidHeader(&'static str),
    /// Checksum mismatch.
    BadChecksum,
    /// Module path too long.
    ModuleNameTooLong,
}

/// A module to load alongside the kernel (e.g. initramfs).
#[derive(Debug, Clone)]
pub struct Module {
    /// Path string passed to the kernel via MB2 module tag.
    pub name: alloc::string::String,
    /// Load address (physical).
    pub addr: u64,
    /// Size in bytes.
    pub size: u64,
}

/// The complete MB2 header: fixed part + tags.
#[derive(Debug, Clone)]
pub struct Mb2Header {
    /// Magic (always MB2_BOOT_MAGIC).
    pub magic: u32,
    /// Architecture.
    pub arch: u32,
    /// Total header length including tags, 8-aligned.
    pub header_length: u32,
    /// checksum = -(magic+arch+len+checksum) mod 2^32 == 0
    pub checksum: u32,
    /// Serialized header tags.
    pub tags: Vec<u8>,
}

impl Mb2Header {
    /// Build a header requesting standard SigmaOS boot info.
    pub fn sigmaos_default(kernel_addr: u64, modules: &[Module]) -> Result<Self, BootError> {
        let mut tags = Vec::new();

        // Information request: mem map, framebuffer, cmdline, boot loader name
        push_info_request(&mut tags, info_flags::MEM_MAP | info_flags::FRAMEBUFFER
            | info_flags::CMDLINE | info_flags::BOOT_LOADER_NAME);

        // Framebuffer request: width/height any, depth 32
        push_tag(&mut tags, tag_type::FRAMEBUFFER, &fb_request_body());

        // Kernel address tag (where MB2 expects the kernel)
        push_tag(&mut tags, tag_type::ADDRESS, &addr_body(kernel_addr));

        // Module tags: one per module
        for m in modules {
            let mut body = Vec::new();
            body.extend_from_slice(&m.addr.to_le_bytes());
            body.extend_from_slice(&m.size.to_le_bytes());
            body.extend_from_slice(&0u32.to_le_bytes()); // string offset (patched later)
            body.extend_from_slice(m.name.as_bytes());
            body.push(0);
            push_tag(&mut tags, tag_type::MODULE, &body);
        }

        // End tag
        push_tag(&mut tags, tag_type::END, &[]);

        let header_length = (8 + tags.len() as u32 + 7) & !7;
        let mut h = Self {
            magic: MB2_BOOT_MAGIC,
            arch: ARCH_I386,
            header_length,
            checksum: 0,
            tags,
        };
        h.checksum = h.compute_checksum();
        Ok(h)
    }

    /// checksum such that magic+arch+len+checksum == 0 mod 2^32.
    fn compute_checksum(&self) -> u32 {
        let sum = self.magic
            .wrapping_add(self.arch)
            .wrapping_add(self.header_length)
            .wrapping_add(0u32);
        0u32.wrapping_sub(sum)
    }

    /// Verify the header checksum is self-consistent.
    pub fn verify(&self) -> Result<(), BootError> {
        let sum = self
            .magic
            .wrapping_add(self.arch)
            .wrapping_add(self.header_length)
            .wrapping_add(self.checksum);
        if sum != 0 {
            return Err(BootError::BadChecksum);
        }
        Ok(())
    }

    /// Serialize the complete header (fixed 8 bytes + tags) to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.magic.to_le_bytes());
        out.extend_from_slice(&self.arch.to_le_bytes());
        out.extend_from_slice(&self.header_length.to_le_bytes());
        out.extend_from_slice(&self.checksum.to_le_bytes());
        out.extend_from_slice(&self.tags);
        out
    }
}

fn push_tag(out: &mut Vec<u8>, tag_type: u16, body: &[u8]) {
    let size = 8 + body.len();
    out.extend_from_slice(&tag_type.to_le_bytes());
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(body);
    while !out.len().is_multiple_of(8) {
        out.push(0);
    }
}

fn push_info_request(out: &mut Vec<u8>, flags: u32) {
    // body: request array of u32 tag type ids + optional flags u32
    let mut body = Vec::new();
    for t in [6u32, 8, 1, 2] {
        // mem map=6, framebuffer=8, cmdline=1, boot loader name=2
        body.extend_from_slice(&t.to_le_bytes());
    }
    body.extend_from_slice(&flags.to_le_bytes());
    push_tag(out, tag_type::INFORMATION_REQUEST, &body);
}

fn fb_request_body() -> Vec<u8> {
    // width u32, height u32, depth u8, pad[5]
    let mut b = Vec::new();
    b.extend_from_slice(&0u32.to_le_bytes()); // any width
    b.extend_from_slice(&0u32.to_le_bytes()); // any height
    b.push(32); // depth
    b.extend_from_slice(&[0u8; 5]);
    b
}

fn addr_body(addr: u64) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&addr.to_le_bytes());
    b
}

/// Multiboot1 header — needed for bootloaders that only implement v1
/// (e.g. QEMU's `-kernel` loader). 12-byte header:
/// magic u32 (0x1BADB002), flags u32, checksum u32 where
/// checksum = -(magic + flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mb1Header {
    /// Boot flags (bit 0 = mem info valid, bit 16 = header offset).
    pub flags: u32,
    /// Offset of this header from the start of the file, when
    /// `flags & (1<<16)` is set. 0 = header is at file offset 0.
    pub header_offset: u32,
}

impl Mb1Header {
    /// MB1 magic value.
    pub const MAGIC: u32 = 0x1BAD_B002;

    /// Create a header at the given file offset.
    pub fn new(header_offset: u32) -> Self {
        // bit 16 set → header offset field is used
        Self { flags: 1 << 16, header_offset }
    }

    /// Serialize to 12 bytes (or 16 with offset field).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(16);
        b.extend_from_slice(&Self::MAGIC.to_le_bytes());
        b.extend_from_slice(&self.flags.to_le_bytes());
        let mut sum = Self::MAGIC.wrapping_add(self.flags);
        b.extend_from_slice(&0u32.to_le_bytes()); // checksum placeholder
        if self.flags & (1 << 16) != 0 {
            b.extend_from_slice(&self.header_offset.to_le_bytes());
            sum = sum.wrapping_add(self.header_offset);
        }
        let checksum = 0u32.wrapping_sub(sum);
        b[8..12].copy_from_slice(&checksum.to_le_bytes());
        b
    }

    /// Verify checksum.
    pub fn verify(&self, bytes: &[u8]) -> bool {
        if bytes.len() < 12 {
            return false;
        }
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if magic != Self::MAGIC {
            return false;
        }
        let flags = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let checksum = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let mut sum = magic.wrapping_add(flags).wrapping_add(checksum);
        if flags & (1 << 16) != 0 {
            if bytes.len() < 16 {
                return false;
            }
            sum = sum.wrapping_add(u32::from_le_bytes(bytes[12..16].try_into().unwrap()));
        }
        sum == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn header_checksum_verifies() {
        let h = Mb2Header::sigmaos_default(0x100000, &[]).unwrap();
        assert_eq!(h.verify(), Ok(()));
        assert_eq!(h.magic, MB2_BOOT_MAGIC);
    }

    #[test]
    fn tampered_checksum_fails() {
        let mut h = Mb2Header::sigmaos_default(0x100000, &[]).unwrap();
        h.checksum ^= 1;
        assert_eq!(h.verify(), Err(BootError::BadChecksum));
    }

    #[test]
    fn header_is_aligned() {
        let h = Mb2Header::sigmaos_default(0x100000, &[]).unwrap();
        assert_eq!(h.header_length % 8, 0);
        let bytes = h.to_bytes();
        assert!(bytes.len() >= 8);
    }

    #[test]
    fn module_tags_present() {
        let modules = vec![Module {
            name: "initramfs.cpio".to_string(),
            addr: 0x200000,
            size: 0x10000,
        }];
        let h = Mb2Header::sigmaos_default(0x100000, &modules).unwrap();
        let bytes = h.to_bytes();
        // module name should appear in serialized tags
        assert!(bytes.windows(14).any(|w| w == b"initramfs.cpio"));
    }

    #[test]
    fn end_tag_is_last() {
        let h = Mb2Header::sigmaos_default(0x100000, &[]).unwrap();
        let bytes = h.to_bytes();
        // last 8 bytes should be end tag (type 0, size 8)
        // tag wire format: type u16, size u32, body, 8-byte aligned
        let n = bytes.len();
        assert_eq!(&bytes[n - 8..n - 6], &0u16.to_le_bytes());
        assert_eq!(&bytes[n - 6..n - 2], &8u32.to_le_bytes());
    }

    #[test]
    fn mb1_header_checksum() {
        let h = Mb1Header::new(0x1000);
        let bytes = h.to_bytes();
        assert_eq!(&bytes[0..4], &0x1BAD_B002u32.to_le_bytes());
        assert!(h.verify(&bytes));
        // tamper → fail
        let mut bad = bytes.clone();
        bad[4] ^= 0xff;
        assert!(!h.verify(&bad));
    }

    #[test]
    fn mb1_header_at_zero() {
        // header at file offset 0 still uses offset field form
        let h = Mb1Header::new(0);
        assert!(h.verify(&h.to_bytes()));
    }
}
