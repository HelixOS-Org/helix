//! # x86_64 Paging
//!
//! Simple paging utilities for userspace support.
//! We modify the existing identity-mapped page tables created by the bootloader.

use core::arch::asm;

/// Page table entry flags
pub mod flags {
    /// Page is present in physical memory
    pub const PRESENT: u64 = 1 << 0;
    /// Page is writable
    pub const WRITABLE: u64 = 1 << 1;
    /// Page is accessible from Ring 3 (user mode)
    pub const USER: u64 = 1 << 2;
    /// This is a huge page (2MB or 1GB)
    pub const HUGE_PAGE: u64 = 1 << 7;
}

/// Get the current CR3 value (page table base)
pub fn get_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    }
    cr3
}

/// Invalidate a TLB entry
pub fn invlpg(addr: u64) {
    unsafe {
        asm!("invlpg [{}]", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Make a memory region accessible from Ring 3 (user mode)
///
/// This function walks the existing page tables and sets the USER bit
/// on all levels (PML4, PDPT, PD, PT) for the given address range.
///
/// # Safety
/// - The address range must be valid and mapped
/// - Must be called from Ring 0
pub unsafe fn make_user_accessible(start_addr: u64, size: usize) {
    unsafe {
        let end_addr = start_addr + size as u64;
        let mut addr = start_addr & !0xFFF; // Page-align
        
        let cr3 = get_cr3();
        let pml4_base = (cr3 & !0xFFF) as *mut u64;
        
        // Debug output
        let msg = b"[PAGING] Making user accessible: ";
        for &c in msg {
            asm!("out dx, al", in("dx") 0x3F8u16, in("al") c, options(nomem, nostack));
        }
        print_hex(start_addr);
        let msg2 = b" to ";
        for &c in msg2 {
            asm!("out dx, al", in("dx") 0x3F8u16, in("al") c, options(nomem, nostack));
        }
        print_hex(end_addr);
        asm!("out dx, al", in("dx") 0x3F8u16, in("al") b'\n', options(nomem, nostack));
        
        while addr < end_addr {
            // Walk the 4-level page table
            let pml4_idx = ((addr >> 39) & 0x1FF) as usize;
            let pdpt_idx = ((addr >> 30) & 0x1FF) as usize;
            let pd_idx = ((addr >> 21) & 0x1FF) as usize;
            let pt_idx = ((addr >> 12) & 0x1FF) as usize;
            
            // Read and modify PML4 entry
            let pml4_entry = pml4_base.add(pml4_idx).read_volatile();
            if pml4_entry & flags::PRESENT == 0 {
                // Not mapped, skip
                addr += 0x1000;
                continue;
            }
            // Set USER bit on PML4 entry
            pml4_base.add(pml4_idx).write_volatile(pml4_entry | flags::USER);
            
            // Read and modify PDPT entry  
            let pdpt_base = ((pml4_entry & !0xFFF) & 0x000F_FFFF_FFFF_F000) as *mut u64;
            let pdpt_entry = pdpt_base.add(pdpt_idx).read_volatile();
            if pdpt_entry & flags::PRESENT == 0 {
                addr += 0x1000;
                continue;
            }
            // Set USER bit on PDPT entry
            pdpt_base.add(pdpt_idx).write_volatile(pdpt_entry | flags::USER);
            
            // Check for 1GB huge page
            if pdpt_entry & flags::HUGE_PAGE != 0 {
                // 1GB page, already done
                invlpg(addr);
                addr += 0x4000_0000; // 1GB
                continue;
            }
            
            // Read and modify PD entry
            let pd_base = ((pdpt_entry & !0xFFF) & 0x000F_FFFF_FFFF_F000) as *mut u64;
            let pd_entry = pd_base.add(pd_idx).read_volatile();
            if pd_entry & flags::PRESENT == 0 {
                addr += 0x1000;
                continue;
            }
            // Set USER bit on PD entry
            pd_base.add(pd_idx).write_volatile(pd_entry | flags::USER);
            
            // Check for 2MB huge page
            if pd_entry & flags::HUGE_PAGE != 0 {
                // 2MB page, already done
                invlpg(addr);
                addr += 0x20_0000; // 2MB
                continue;
            }
            
            // Read and modify PT entry
            let pt_base = ((pd_entry & !0xFFF) & 0x000F_FFFF_FFFF_F000) as *mut u64;
            let pt_entry = pt_base.add(pt_idx).read_volatile();
            if pt_entry & flags::PRESENT == 0 {
                addr += 0x1000;
                continue;
            }
            // Set USER bit on PT entry
            pt_base.add(pt_idx).write_volatile(pt_entry | flags::USER);
            
            // Flush TLB for this page
            invlpg(addr);
            addr += 0x1000; // 4KB
        }
        
        let msg = b"[PAGING] User pages configured\n";
        for &c in msg {
            asm!("out dx, al", in("dx") 0x3F8u16, in("al") c, options(nomem, nostack));
        }
    }
}

/// Print a hex value to serial
unsafe fn print_hex(val: u64) {
    unsafe {
        for i in (0..16).rev() {
            let nibble = ((val >> (i * 4)) & 0xF) as u8;
            let ch = if nibble < 10 { b'0' + nibble } else { b'a' + nibble - 10 };
            asm!("out dx, al", in("dx") 0x3F8u16, in("al") ch, options(nomem, nostack));
        }
    }
}
