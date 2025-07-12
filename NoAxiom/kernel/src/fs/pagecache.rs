use alloc::{collections::vec_deque::VecDeque, sync::Arc, vec::Vec};
use core::intrinsics::unlikely;

use arch::{Arch, ArchInt};
use config::{fs::PAGE_CACHE_PROPORTION, mm::PAGE_SIZE};
use hashbrown::HashMap;
use kfuture::block::block_on;
use lazy_static::lazy_static;
use memory::frame::{frame_alloc, FrameTracker, FRAME_ALLOCATOR};
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard}; // FIXME: use ksync::mutex

use crate::{
    fs::vfs::{
        basic::{dentry::Dentry, file::File},
        root_dentry,
    },
    utils::{global_alloc, is_aligned},
};

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
    file: Arc<dyn File>,
    offset_align: usize,
}

impl Page {
    pub fn new(dentry: Arc<dyn Dentry>, offset_align: usize, state: PageState) -> Self {
        debug!(
            "[Page::new] create new page: {}, offset: {}",
            dentry.name(),
            offset_align
        );
        Self {
            data: frame_alloc().unwrap(),
            state,
            file: dentry.open().unwrap(),
            offset_align,
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

    fn sync(&mut self) {
        match self.state {
            PageState::Modified => {
                self.state = PageState::Shared;
                // debug!(
                //     "[Page::sync] sync page: {}, offset: {}, content: {:?}",
                //     self.file.name(),
                //     self.offset_align,
                //     &self.as_mut_bytes_array()[..10]
                // );
                assert_ne!(self.file.name(), "ForPageCacheManager");
                let file = self.file.clone();
                assert_no_lock!();
                assert!(Arch::is_external_interrupt_enabled());
                let size = self.file.size();
                let len = PAGE_SIZE.min(size - self.offset_align);
                block_on(file.base_write(self.offset_align, &self.as_mut_bytes_array()[..len]))
                    .unwrap();
            }
            _ => {}
        }
    }
}

const PAGE_CACHE_CAPACITY_UNINITIALIZED: usize = 0;
static mut PAGE_CACHE_CAPACITY: usize = PAGE_CACHE_CAPACITY_UNINITIALIZED;
fn get_page_cache_capacity() -> usize {
    unsafe {
        if unlikely(PAGE_CACHE_CAPACITY == PAGE_CACHE_CAPACITY_UNINITIALIZED) {
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
fn get_page_cache_capacity_clean_threshold() -> usize {
    get_page_cache_capacity() / 2
}

struct PageWrapper {
    valid: bool,
    page: Page,
    cache_id: usize, // the id in PageCacheManager
}

impl PageWrapper {
    pub fn new(page: Page) -> Self {
        Self {
            valid: false,
            page,
            cache_id: 0,
        }
    }
    pub fn from(page: Page, cache_id: usize) -> Self {
        Self {
            valid: true,
            page,
            cache_id,
        }
    }
}

pub struct PageCacheManager {
    data: Vec<PageWrapper>,
    free_page: VecDeque<usize>,
}

impl PageCacheManager {
    pub fn new() -> Self {
        let capacity = get_page_cache_capacity();
        let mut data = Vec::with_capacity(capacity);
        let mut free_page = VecDeque::with_capacity(capacity);
        for i in 0..capacity {
            let new_page = Page::new(root_dentry().clone(), 0, PageState::Invalid);
            data.push(PageWrapper::new(new_page));
            free_page.push_back(i);
        }
        Self { data, free_page }
    }

    // todo: use more efficient strategy to clean
    fn clean(&mut self, thresold: usize) {
        warn!("PageCacheManager::clean");
        assert!(
            thresold <= self.free_page.len(),
            "[PageCacheManager::clean] thresold should be less than free_page.len()"
        );
        let mut size = 0;
        for i in 0..self.data.len() {
            if self.data[i].valid {
                self.free_page.push_back(i);
                self.data[i].page.sync();
                self.data[i].valid = false;
                size += 1;
            }
            if size == thresold {
                break;
            }
        }
        assert!(
            size == thresold,
            "[PageCacheManager::clean] clean size should be less than PAGE_CACHE_CLEAN_THRESHOLD"
        );
    }

    fn alloc(&mut self) -> usize {
        // if not full
        if let Some(page) = self.free_page.pop_front() {
            return page;
        }

        let sys_capacity = get_page_cache_capacity();
        let sys_thresold = get_page_cache_capacity_clean_threshold();
        let cur_capacity = self.data.len();
        if cur_capacity < sys_capacity {
            for _ in cur_capacity..sys_capacity {
                let new_page = Page::new(root_dentry().clone(), 0, PageState::Invalid);
                self.data.push(PageWrapper::new(new_page));
                self.free_page.push_back(self.data.len() - 1);
            }
            return self.alloc();
        }

        // if full, do clean
        self.clean(sys_thresold);
        self.alloc()
    }

    pub fn alloc_fill(&mut self, page: Page, cache_id: usize) -> usize {
        let page_id = self.alloc();
        assert_eq!(
            self.data[page_id].valid, false,
            "[PageCache::alloc] Page should not be valid"
        );
        // debug!(
        //     "[PageCacheManager::alloc_fill] alloc page: {}, offset: {}, content:
        // {:?}",     page.file.name(),
        //     page.offset_align,
        //     &page.as_mut_bytes_array()[..10]
        // );
        self.data[page_id] = PageWrapper::from(page, cache_id);
        assert_eq!(
            self.data[page_id].valid, true,
            "[PageCache::alloc_fill] Page should be valid after alloc_fill"
        );
        page_id
    }

    pub fn get_page(&self, page_id: usize, cache_id: usize, offset_align: usize) -> Option<&Page> {
        if self.data[page_id].valid
            && self.data[page_id].cache_id == cache_id
            && self.data[page_id].page.offset_align == offset_align
        {
            // debug!(
            //     "[PageCacheManager::get_page] get page: {}, offset: {}, content: {:?}",
            //     self.data[page_id].page.file.name(),
            //     self.data[page_id].page.offset_align,
            //     &self.data[page_id].page.as_mut_bytes_array()[..10]
            // );
            Some(&self.data[page_id].page)
        } else {
            None
        }
    }

    pub fn get_page_mut(
        &mut self,
        page_id: usize,
        cache_id: usize,
        offset_align: usize,
    ) -> Option<&mut Page> {
        if self.data[page_id].valid
            && self.data[page_id].cache_id == cache_id
            && self.data[page_id].page.offset_align == offset_align
        {
            debug!(
                "[PageCacheManager::get_page_mut] get page: {}, offset: {}, content:
            {:?}",
                self.data[page_id].page.file.name(),
                self.data[page_id].page.offset_align,
                &self.data[page_id].page.as_mut_bytes_array()[..10]
            );
            Some(&mut self.data[page_id].page)
        } else {
            None
        }
    }
}

/// Page cache for filesystem, which should be covered in [`Lock`]
pub struct PageCache {
    pub id: usize,
    inner: HashMap<usize, usize>,
}

impl PageCache {
    /// Create a new page cache
    pub fn new() -> Self {
        Self {
            id: global_alloc() as usize,
            inner: HashMap::new(),
        }
    }

    pub fn fill_page(&mut self, offset_align: usize, page: Page) {
        assert!(is_aligned(offset_align, PAGE_SIZE));
        self.inner.insert(
            offset_align,
            PAGE_CACHE_MANAGER.write().alloc_fill(page, self.id),
        );
    }

    pub fn get_page(
        &self,
        offset_align: usize,
    ) -> Option<(RwLockReadGuard<'_, PageCacheManager>, usize)> {
        assert!(is_aligned(offset_align, PAGE_SIZE));
        if let Some(page_id) = self.inner.get(&offset_align) {
            Some((PAGE_CACHE_MANAGER.read(), *page_id))
        } else {
            None
        }
    }

    pub fn get_page_mut(
        &mut self,
        offset_align: usize,
    ) -> Option<(RwLockWriteGuard<'_, PageCacheManager>, usize)> {
        assert!(is_aligned(offset_align, PAGE_SIZE));
        if let Some(page_id) = self.inner.get(&offset_align) {
            Some((PAGE_CACHE_MANAGER.write(), *page_id))
        } else {
            None
        }
    }

    pub fn mark_deleted(&mut self) {
        // mark all pages as deleted
        for (offset_align, page_id) in self.inner.iter() {
            if let Some(page) =
                PAGE_CACHE_MANAGER
                    .write()
                    .get_page_mut(*page_id, self.id, *offset_align)
            {
                page.mark_deleted();
            }
        }
    }
    /*
    FIXME: if some of the page is cleand, the mark_deleted will not cover all the page. Maybe is fine?
     */
}
