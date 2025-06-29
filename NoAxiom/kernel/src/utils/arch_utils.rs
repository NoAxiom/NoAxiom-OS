use arch::{MappingFlags, TrapType};

/// mismatch prot
pub fn flag_match_with_trap_type(flag: MappingFlags, trap_type: TrapType) -> bool {
    match trap_type {
        TrapType::LoadPageFault(_) => flag.contains(MappingFlags::R),
        TrapType::StorePageFault(_) => flag.contains(MappingFlags::W),
        TrapType::InstructionPageFault(_) => flag.contains(MappingFlags::X),
        _ => false,
    }
}
