//! Provides functions to read and write segment registers.

use crate::x86_64::structures::gdt::SegmentSelector;

/// Reload code segment register.
/// Note this is special since we can not directly move
/// to %cs. Instead we push the new segment selector
/// and return value on the stack and use lretq
/// to reload cs and continue at 1:.
pub unsafe fn set_cs(sel: SegmentSelector) {

    #[cfg(target_arch="x86")]
    #[inline(always)]
    unsafe fn inner(sel: SegmentSelector) {
        llvm_asm!("pushl $0; \
              pushl $$1f; \
              lretl; \
              1:" :: "ri" (u64::from(sel.0)) : "rax" "memory");
    }

    #[cfg(target_arch="x86_64")]
    #[inline(always)]
    unsafe fn inner(sel: SegmentSelector) {
        llvm_asm!("pushq $0; \
              leaq  1f(%rip), %rax; \
              pushq %rax; \
              lretq; \
              1:" :: "ri" (u64::from(sel.0)) : "rax" "memory");
    }

    inner(sel)
}

/// Reload stack segment register.
pub unsafe fn load_ss(sel: SegmentSelector) {
    llvm_asm!("movw $0, %ss " :: "r" (sel.0) : "memory");
}

/// Reload data segment register.
pub unsafe fn load_ds(sel: SegmentSelector) {
    llvm_asm!("movw $0, %ds " :: "r" (sel.0) : "memory");
}

/// Reload es segment register.
pub unsafe fn load_es(sel: SegmentSelector) {
    llvm_asm!("movw $0, %es " :: "r" (sel.0) : "memory");
}

/// Reload fs segment register.
pub unsafe fn load_fs(sel: SegmentSelector) {
    llvm_asm!("movw $0, %fs " :: "r" (sel.0) : "memory");
}

/// Reload gs segment register.
pub unsafe fn load_gs(sel: SegmentSelector) {
    llvm_asm!("movw $0, %gs " :: "r" (sel.0) : "memory");
}

/// Returns the current value of the code segment register.
pub fn cs() -> SegmentSelector {
    let segment: u16;
    unsafe { llvm_asm!("mov %cs, $0" : "=r" (segment) ) };
    SegmentSelector(segment)
}
