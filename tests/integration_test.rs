//! Integration tests — bootloader header + kernel boot-info contract.

use sigma_bootloader::{info_flags, tag_type, Mb2Header, Module};

#[test]
fn header_verifies_and_serializes() {
    let h = Mb2Header::sigmaos_default(0x100000, &[]).unwrap();
    h.verify().unwrap();
    let bytes = h.to_bytes();
    // fixed part: magic, arch, header_length, checksum
    assert_eq!(u32::from_le_bytes(bytes[0..4].try_into().unwrap()), 0x1BAD_B002);
    assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 0);
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), h.header_length);
    assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), h.checksum);
}

#[test]
fn requests_kernel_needed_tags() {
    // info request tag (type 1) must ask for mem map + framebuffer
    let h = Mb2Header::sigmaos_default(0x100000, &[]).unwrap();
    let bytes = h.to_bytes();
    let needle = info_flags::MEM_MAP
        | info_flags::FRAMEBUFFER
        | info_flags::CMDLINE
        | info_flags::BOOT_LOADER_NAME;
    let found = bytes
        .windows(4)
        .any(|w| u32::from_le_bytes(w.try_into().unwrap()) == needle);
    assert!(found, "info request flags missing");
}

#[test]
fn module_roundtrip() {
    let modules = vec![Module {
        name: "initramfs.cpio".to_string(),
        addr: 0x300000,
        size: 0x8000,
    }];
    let h = Mb2Header::sigmaos_default(0x100000, &modules).unwrap();
    let bytes = h.to_bytes();
    assert!(bytes.windows(4).any(|w| u16::from_le_bytes([w[0], w[1]]) == tag_type::MODULE));
}
