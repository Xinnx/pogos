use super::entry::*;
use super::table::{self, Level4, Table};
use super::{Page, PhysicalAddress, VirtualAddress, ENTRY_COUNT};
use core::ptr::Unique;
use crate::memory::{Frame, FrameAllocator, PAGE_SIZE};

pub struct Mapper {
    p4: Unique<Table<Level4>>,
}

impl Mapper {
    pub unsafe fn new() -> Mapper {
        Mapper {
            p4: Unique::new_unchecked(table::P4),
        }
    }

    pub fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    pub fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }

    pub fn translate_page(&self, page: Page) -> Option<Frame> {
        let p3 = self.p4().next_table(page.p4_index());

        let huge_page = || {
            p3.and_then(|p3| {
                let p3_entry = &p3[page.p3_index() as usize];
                // 1GiB page?
                if let Some(start_frame) = p3_entry.pointed_frame() {
                    if p3_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                        // address must be 1GiB aligned
                        assert!(start_frame.number % (ENTRY_COUNT * ENTRY_COUNT) == 0);
                        return Some(Frame {
                            number: start_frame.number
                                + page.p2_index() * ENTRY_COUNT
                                + page.p1_index(),
                        });
                    }
                }
                if let Some(p2) = p3.next_table(page.p3_index()) {
                    let p2_entry = &p2[page.p2_index() as usize];
                    // 2MiB page?
                    if let Some(start_frame) = p2_entry.pointed_frame() {
                        if p2_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                            // address must be 2MiB aligned
                            assert!(start_frame.number % ENTRY_COUNT == 0);
                            return Some(Frame {
                                number: start_frame.number + page.p1_index(),
                            });
                        }
                    }
                }
                None
            })
        };

        p3.and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1[page.p1_index() as usize].pointed_frame())
            .or_else(huge_page)
    }

    // Maps the page to the frame with the provided flags.
    /// The `PRESENT` flag is added by default. Needs a
    /// `FrameAllocator` as it might need to create new page tables.
    pub fn map_to<A>(&mut self, page: Page, frame: Frame, flags: EntryFlags, alloc: &mut A)
    where
        A: FrameAllocator,
    {
        let p3 = self.p4_mut().next_table_create(page.p4_index(), alloc);
        let p2 = p3.next_table_create(page.p3_index(), alloc);
        let p1 = p2.next_table_create(page.p2_index(), alloc);

        assert!(p1[page.p1_index() as usize].is_unused());
        p1[page.p1_index() as usize].set(frame, flags | EntryFlags::PRESENT);
    }

    /// Translates a virtual to the corresponding physical address.
    /// Returns `None` if the address is not mapped.
    pub fn translate(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = virtual_address % PAGE_SIZE;
        self.translate_page(Page::containing_address(virtual_address))
            .map(|frame| frame.number * PAGE_SIZE + offset)
    }

    /// Maps the page to some free frame with the provided flags.
    /// The free frame is allocated from the given `FrameAllocator`.
    pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        let frame = allocator.allocate_frame().expect("out of memory");
        self.map_to(page, frame, flags, allocator)
    }

    /// Unmaps the given page and adds all freed frames to the given
    /// `FrameAllocator`.
    pub fn unmap<A>(&mut self, page: Page, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        assert!(self.translate(page.start_address()).is_some());

        let p1 = self
            .p4_mut()
            .next_table_mut(page.p4_index())
            .and_then(|p3| p3.next_table_mut(page.p3_index()))
            .and_then(|p2| p2.next_table_mut(page.p2_index()))
            .expect("mapping code does not support huge pages");

        let frame = p1[page.p1_index() as usize].pointed_frame().unwrap();

        p1[page.p1_index() as usize].set_unused();

        // TLB needs to be flushed after page table updates
        use crate::x86_64::instructions::tlb;
        use crate::x86_64::VirtualAddress;
        tlb::flush(VirtualAddress(page.start_address() as usize));

        // TODO free p(1,2,3) table if empty
        // TODO: deallocate implementation
        //allocator.deallocate_frame(frame);
    }

    /// Identity map the the given frame with the provided flags.
    /// The `FrameAllocator` is used to create new page tables if needed.
    pub fn identity_map<A>(&mut self, frame: Frame, flags: EntryFlags, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        let page = Page::containing_address(frame.start_address());
        self.map_to(page, frame, flags, allocator)
    }
}
