use crate::{MappingFlags, PageFaultType};

/// mismatch prot
pub fn flag_match_with_trap_type(flag: MappingFlags, trap_type: PageFaultType) -> bool {
    match trap_type {
        PageFaultType::LoadPageFault(_) => flag.contains(MappingFlags::R),
        PageFaultType::StorePageFault(_) => flag.contains(MappingFlags::W),
        PageFaultType::InstructionPageFault(_) => flag.contains(MappingFlags::X),
        _ => false,
    }
}
