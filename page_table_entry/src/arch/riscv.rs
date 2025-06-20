//! RISC-V page table entries.

use core::fmt;
use memory_addr::PhysAddr;

use crate::{GenericPTE, MappingFlags};

bitflags::bitflags! {
    /// Page-table entry flags.
    #[derive(Debug)]
    pub struct PTEFlags: usize {
        /// Whether the PTE is valid.
        const V =   1 << 0;
        /// Whether the page is readable.
        const R =   1 << 1;
        /// Whether the page is writable.
        const W =   1 << 2;
        /// Whether the page is executable.
        const X =   1 << 3;
        /// Whether the page is accessible to user mode.
        const U =   1 << 4;
        /// Designates a global mapping.
        const G =   1 << 5;
        /// Indicates the virtual page has been read, written, or fetched from
        /// since the last time the A bit was cleared.
        const A =   1 << 6;
        /// Indicates the virtual page has been written since the last time the
        /// D bit was cleared.
        const D =   1 << 7;
        /// Two bits reserved for supervisor software, e.g. COW
        const RSW1 = 1 << 8;
        const RSW2 = 1 << 9;
    }
}

impl From<PTEFlags> for MappingFlags {
    fn from(f: PTEFlags) -> Self {
        let mut ret = Self::empty();
        if !f.contains(PTEFlags::V) {
            return ret;
        }
        if f.contains(PTEFlags::R) {
            ret |= Self::READ;
        }
        if f.contains(PTEFlags::W) {
            ret |= Self::WRITE;
        }
        if f.contains(PTEFlags::X) {
            ret |= Self::EXECUTE;
        }
        if f.contains(PTEFlags::U) {
            ret |= Self::USER;
        }
        #[cfg(feature = "COW")]
        if f.contains(PTEFlags::RSW1) {
            ret |= Self::COW;
        }
        ret
    }
}

impl From<MappingFlags> for PTEFlags {
    fn from(f: MappingFlags) -> Self {
        if f.is_empty() {
            return Self::empty();
        }
        let mut ret = Self::V;
        if f.contains(MappingFlags::READ) {
            ret |= Self::R;
        }
        if f.contains(MappingFlags::WRITE) {
            ret |= Self::W;
        }
        if f.contains(MappingFlags::EXECUTE) {
            ret |= Self::X;
        }
        if f.contains(MappingFlags::USER) {
            ret |= Self::U;
        }
        #[cfg(feature = "COW")]
        if f.contains(MappingFlags::COW) {
            ret |= Self::RSW1;
        }
        ret
    }
}

/// Sv39 and Sv48 page table entry for RV64 systems.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Rv64PTE(u64);

impl Rv64PTE {
    const PHYS_ADDR_MASK: u64 = (1 << 54) - (1 << 10); // bits 10..54

    /// Creates an empty descriptor with all bits set to zero.
    pub const fn empty() -> Self {
        Self(0)
    }
}

impl GenericPTE for Rv64PTE {
    fn new_page(paddr: PhysAddr, flags: MappingFlags, _is_huge: bool) -> Self {
        let flags = PTEFlags::from(flags) | PTEFlags::A | PTEFlags::D;
        debug_assert!(flags.intersects(PTEFlags::R | PTEFlags::X));
        Self(flags.bits() as u64 | ((paddr.as_usize() >> 2) as u64 & Self::PHYS_ADDR_MASK))
    }
    fn new_table(paddr: PhysAddr) -> Self {
        Self(PTEFlags::V.bits() as u64 | ((paddr.as_usize() >> 2) as u64 & Self::PHYS_ADDR_MASK))
    }
    fn paddr(&self) -> PhysAddr {
        PhysAddr::from(((self.0 & Self::PHYS_ADDR_MASK) << 2) as usize)
    }
    fn flags(&self) -> MappingFlags {
        PTEFlags::from_bits_truncate(self.0 as usize).into()
    }
    fn set_paddr(&mut self, paddr: PhysAddr) {
        self.0 = (self.0 & !Self::PHYS_ADDR_MASK)
            | ((paddr.as_usize() as u64 >> 2) & Self::PHYS_ADDR_MASK);
    }
    fn set_flags(&mut self, flags: MappingFlags, _is_huge: bool) {
        let flags = PTEFlags::from(flags) | PTEFlags::A | PTEFlags::D;
        debug_assert!(flags.intersects(PTEFlags::R | PTEFlags::X));
        self.set_flags_arch(flags)
    }

    fn set_flags_arch(&mut self, flags: PTEFlags) {
        self.0 = (self.0 & Self::PHYS_ADDR_MASK) | flags.bits() as u64;
    }

    fn bits(self) -> usize {
        self.0 as usize
    }
    fn is_unused(&self) -> bool {
        self.0 == 0
    }
    fn is_present(&self) -> bool {
        PTEFlags::from_bits_truncate(self.0 as usize).contains(PTEFlags::V)
    }
    fn is_dirty(&self) -> bool {
        PTEFlags::from_bits_truncate(self.0 as usize).contains(PTEFlags::D)
    }
    fn set_dirty(&mut self, dirty: bool) {
        if dirty {
            self.0 |= PTEFlags::D.bits() as u64;
        } else {
            self.0 &= !(PTEFlags::D.bits() as u64);
        }
    }
    fn is_accessed(&self) -> bool {
        PTEFlags::from_bits_truncate(self.0 as usize).contains(PTEFlags::A)
    }
    fn set_accessed(&mut self, accessed: bool) {
        if accessed {
            self.0 |= PTEFlags::A.bits() as u64;
        } else {
            self.0 &= !(PTEFlags::A.bits() as u64);
        }
    }
    fn is_huge(&self) -> bool {
        PTEFlags::from_bits_truncate(self.0 as usize).intersects(PTEFlags::R | PTEFlags::X)
    }
    fn clear(&mut self) {
        self.0 = 0
    }
}

impl fmt::Debug for Rv64PTE {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("Rv64PTE");
        f.field("raw", &self.0)
            .field("paddr", &self.paddr())
            .field("flags", &self.flags())
            .finish()
    }
}
