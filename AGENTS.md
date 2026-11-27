# AGENTS — SigmaOS-sigma-bootloader

SigmaOS stage-1/stage-2 bootloader and boot protocol.

Automated agents working on this component must:
- Maintain `#![forbid(unsafe_code)]` where applicable (except low-level kernel/MM/driver modules).
- Keep all code and documentation SigmaOS-native (no foreign distro references).
- Run `cargo test --offline` (with `--lib` for `#![no_std]` kernel crates) before committing.
- Keep `COMPONENT.toml`, `README.md` (`## See Also`), and `tools-sigmaos.sh` up to date.
