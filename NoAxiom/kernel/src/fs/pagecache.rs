use alloc::{
    collections::vec_deque::VecDeque,
    sync::{Arc, Weak},
    vec::Vec,
};
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
        basic::{
            dentry::{Dentry, EmptyDentry},
            file::File,
            inode::InodeState,
        },
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
                //     "[Page::sync] sync page: {}, offset: {}",
                //     self.file.name(),
                //     self.offset_align
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

static mut PAGE_CACHE_CAPACITY: usize = 0;
fn get_page_cache_capacity() -> usize {
    unsafe {
        if unlikely(PAGE_CACHE_CAPACITY == 0) {
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
    data: Vec<PageWrapper>, // (Page, valid) // todo: (Page, valid, old)
    free_page: VecDeque<usize>,
}

impl PageCacheManager {
    pub fn new() -> Self {
        let mut data = Vec::with_capacity(get_page_cache_capacity());
        let mut free_page = VecDeque::with_capacity(get_page_cache_capacity());
        for i in 0..get_page_cache_capacity() {
            let new_page = Page::new(root_dentry().clone(), 0, PageState::Invalid);
            data.push(PageWrapper::new(new_page));
            free_page.push_back(i);
        }
        Self { data, free_page }
    }

    // todo: use more efficient strategy to clean
    fn clean(&mut self) {
        error!("PageCacheManager::clean");
        let mut size = 0;
        for i in 0..self.data.len() {
            if self.data[i].valid {
                self.free_page.push_back(i);
                self.data[i].page.sync();
                self.data[i].valid = false;
                size += 1;
            }
            if size >= get_page_cache_capacity_clean_threshold() {
                break;
            }
        }
    }

    fn alloc(&mut self) -> usize {
        // if not full
        if let Some(page) = self.free_page.pop_front() {
            return page;
        }

        // if full, do clean
        self.clean();
        self.alloc()
    }

    pub fn alloc_fill(&mut self, page: Page, cache_id: usize) -> usize {
        let page_id = self.alloc();
        assert_eq!(
            self.data[page_id].valid, false,
            "[PageCache::alloc] Page should not be valid"
        );
        self.data[page_id] = PageWrapper::from(page, cache_id);
        page_id
    }

    pub fn get_page(&self, page_id: usize, cache_id: usize) -> Option<&Page> {
        if self.data[page_id].valid && self.data[page_id].cache_id == cache_id {
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

    pub fn get_page_mut(&mut self, page_id: usize, cache_id: usize) -> Option<&mut Page> {
        if self.data[page_id].valid && self.data[page_id].cache_id == cache_id {
            // debug!(
            //     "[PageCacheManager::get_page_mut] get page: {}, offset: {}, content:
            // {:?}",     self.data[page_id].page.file.name(),
            //     self.data[page_id].page.offset_align,
            //     &self.data[page_id].page.as_mut_bytes_array()[..10]
            // );
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
        for (_, page_id) in self.inner.iter() {
            if let Some(page) = PAGE_CACHE_MANAGER.write().get_page_mut(*page_id, self.id) {
                page.mark_deleted();
            }
        }
    }
    /*
    FIXME: if some of the page is cleand, the mark_deleted will not cover all the page. Maybe is fine?
     */
}
