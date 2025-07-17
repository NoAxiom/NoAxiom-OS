use alloc::{sync::Arc, vec::Vec};
use core::intrinsics::unlikely;

use arch::{Arch, ArchInt};
use config::{fs::PAGE_CACHE_PROPORTION, mm::PAGE_SIZE};
use hashbrown::HashMap;
use kfuture::block::block_on;
use lazy_static::lazy_static;
use memory::frame::{frame_alloc, FrameTracker, FRAME_ALLOCATOR};
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard}; // FIXME: use ksync::mutex

use crate::{fs::vfs::basic::file::File, utils::is_aligned};

const PAGE_CACHE_CAPACITY_UNINITIALIZED: usize = 0;
static mut PAGE_CACHE_CAPACITY: usize = PAGE_CACHE_CAPACITY_UNINITIALIZED;
fn get_page_cache_capacity() -> usize {
    unsafe {
        if unlikely(PAGE_CACHE_CAPACITY == PAGE_CACHE_CAPACITY_UNINITIALIZED) {
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            PAGE_CACHE_CAPACITY = FRAME_ALLOCATOR.lock().stat_total() / PAGE_CACHE_PROPORTION;
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            println_debug!(
                "[kernel] PAGECACHE: init {} physical frames",
                PAGE_CACHE_CAPACITY
            );
        }
        PAGE_CACHE_CAPACITY
    }
}
fn get_page_cache_capacity_clean_threshold(page_cache_capacity: usize) -> usize {
    page_cache_capacity / 2
}

lazy_static! {
    pub static ref PAGE_CACHE_MANAGER: RwLock<PageCacheManager> =
        RwLock::new(PageCacheManager::new());
}

/// Inspired by `MSI`
#[derive(PartialEq)]
pub enum PageState {
    Modified,
    Shared,
    Invalid,
    Deleted,
}

pub struct Page {
    data: FrameTracker,
    state: PageState,
}

impl Page {
    pub fn new(state: PageState) -> Self {
        Self {
            data: frame_alloc().unwrap(),
            state,
        }
    }
    pub fn as_mut_bytes_array(&self) -> &'static mut [u8] {
        self.data.ppn().get_bytes_array()
    }
    pub fn mark_dirty(&mut self) {
        if self.state != PageState::Deleted {
            self.state = PageState::Modified;
        }
    }
    pub fn mark_deleted(&mut self) {
        self.state = PageState::Deleted;
    }
}

pub struct PageCacheManager {
    inner: HashMap<Arc<dyn File>, HashMap<usize, Page>>,
    page_count: usize,
}

impl PageCacheManager {
    fn new() -> Self {
        Self {
            inner: HashMap::new(),
            page_count: 0,
        }
    }

    // todo: use more efficient strategy to clean
    fn clean(&mut self, thresold: usize) {
        error!("[PageCacheManager: clean], thresold: {}", thresold);
        let mut count = 0;
        for (file, page_cache) in self.inner.iter_mut() {
            debug!(
                "[PageCacheManager: clean] file: {}, page_cache size: {}",
                file.name(),
                page_cache.len()
            );
        }
        for (file, page_cache) in self.inner.iter_mut() {
            let file_size = file.size();
            let mut sorted = Vec::new();
            for (offset, page) in page_cache.iter() {
                sorted.push((*offset, page));
            }
            sorted.sort_by_key(|(offset, _)| *offset);

            let mut offsets_to_remove = Vec::new();
            for (offset, page) in &sorted {
                if page.state == PageState::Modified {
                    assert_no_lock!();
                    assert!(Arch::is_external_interrupt_enabled());
                    let len = PAGE_SIZE.min(file_size - offset);
                    block_on(file.base_write(*offset, &page.data.ppn().get_bytes_array()[..len]))
                        .unwrap();
                }
                offsets_to_remove.push(*offset);
                count += 1;
                if count >= thresold {
                    break;
                }
            }
            for offset in offsets_to_remove {
                page_cache.remove(&offset);
            }
            if count >= thresold {
                self.page_count -= count;
                return;
            }
        }
    }

    fn alloc(&mut self, state: PageState) -> Page {
        let sys_capacity = get_page_cache_capacity();
        let sys_thresold = get_page_cache_capacity_clean_threshold(sys_capacity);
        if self.page_count > sys_capacity {
            self.clean(sys_thresold);
        }
        Page::new(state)
    }

    pub fn get_page(&self, file: &Arc<dyn File>, offset_align: usize) -> Option<&Page> {
        if let Some(page_cache) = self.inner.get(file) {
            if let Some(page) = page_cache.get(&offset_align) {
                debug!(
                    "[PageCacheManager: get_page] file: {}, offset: {} succeed, content: {:?}",
                    file.name(),
                    offset_align,
                    &page.as_mut_bytes_array()[..10],
                );
                return Some(page);
            }
        }

        debug!(
            "[PageCacheManager: get_page] file: {}, offset: {} fail",
            file.name(),
            offset_align,
        );
        None
    }

    pub fn get_page_mut(&mut self, file: &Arc<dyn File>, offset_align: usize) -> Option<&mut Page> {
        if let Some(page_cache) = self.inner.get_mut(file) {
            if let Some(page) = page_cache.get_mut(&offset_align) {
                debug!(
                    "[PageCacheManager: get_page_mut] file: {}, offset: {} succeed, content: {:?}",
                    file.name(),
                    offset_align,
                    &page.as_mut_bytes_array()[..10],
                );
                return Some(page);
            }
        }

        debug!(
            "[PageCacheManager: get_page_mut] file: {}, offset: {} fail",
            file.name(),
            offset_align,
        );
        None
    }

    pub fn alloc_page_mut(
        &mut self,
        file: &Arc<dyn File>,
        offset_align: usize,
        state: PageState,
    ) -> &mut Page {
        assert!(is_aligned(offset_align, PAGE_SIZE));
        self.page_count += 1;
        let page = self.alloc(state);
        let page_cache = self.inner.entry(file.clone()).or_insert_with(HashMap::new);
        if let Some(_) = page_cache.get(&offset_align) {
            panic!(
                "[PageCacheManager: alloc_page_mut] file: {}, offset: {} already exists",
                file.name(),
                offset_align
            );
        } else {
            debug!(
                "[PageCacheManager: alloc_page_mut] file: {}, offset: {}",
                file.name(),
                offset_align
            );
            assert!(page_cache.insert(offset_align, page).is_none());
            page_cache
                .get_mut(&offset_align)
                .expect("[PageCacheManager: alloc_page_mut] page not found")
        }
    }

    pub fn mark_deleted(&mut self, file: &Arc<dyn File>) {
        self.inner.get_mut(file).map(|cache| {
            cache.iter_mut().for_each(|(_, page)| {
                page.mark_deleted();
            })
        });
    }
}

#[inline(always)]
pub fn get_pagecache_rguard() -> RwLockReadGuard<'static, PageCacheManager> {
    PAGE_CACHE_MANAGER.read()
}

#[inline(always)]
pub fn get_pagecache_wguard() -> RwLockWriteGuard<'static, PageCacheManager> {
    PAGE_CACHE_MANAGER.write()
}
