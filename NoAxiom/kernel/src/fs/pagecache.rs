use core::{panic, sync::atomic::AtomicUsize};

use config::mm::PAGE_SIZE;
use hashbrown::HashMap;
use memory::frame::{frame_alloc, FrameTracker};

use crate::utils::is_aligned;

/// Inspired by `MSI`
enum PageState {
    Modified,
    Shared,
    Invalid,
}

pub static mut FRAME_ALLOCS: AtomicUsize = AtomicUsize::new(0);

pub struct Page {
    data: FrameTracker,
    state: PageState,
}

impl Page {
    pub fn new() -> Self {
        unsafe { FRAME_ALLOCS.fetch_add(1, core::sync::atomic::Ordering::SeqCst) };
        Self {
            data: frame_alloc().unwrap(),
            state: PageState::Invalid,
        }
    }
    pub fn as_mut_bytes_array(&self) -> &'static mut [u8] {
        self.data.ppn().get_bytes_array()
    }
    pub fn mark_dirty(&mut self) {
        self.state = PageState::Modified;
    }
    #[allow(unused)]
    fn sync(&self) {
        match self.state {
            PageState::Modified => {
                todo!("sync page");
            }
            _ => {}
        }
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        unsafe { FRAME_ALLOCS.fetch_sub(1, core::sync::atomic::Ordering::SeqCst) };
    }
}

/// Page cache for filesystem, which should be covered in [`Lock`]
pub struct PageCache {
    dont_use: bool,
    inner: HashMap<usize, Page>,
}

impl PageCache {
    /// Create a new page cache
    pub fn new(dont_use: bool) -> Self {
        Self {
            dont_use,
            inner: HashMap::new(),
        }
    }

    pub fn fill_page(&mut self, offset_align: usize, page: Page) {
        assert!(is_aligned(offset_align, PAGE_SIZE));
        if let Some(_) = self.inner.get_mut(&offset_align) {
            panic!("page already exists: {:#x}", offset_align);
        } else {
            self.inner.insert(offset_align, page);
        }
    }

    pub fn get_page(&self, offset_align: usize) -> Option<&Page> {
        assert!(is_aligned(offset_align, PAGE_SIZE));
        self.inner.get(&offset_align)
    }

    pub fn get_page_mut(&mut self, offset_align: usize) -> Option<&mut Page> {
        assert!(is_aligned(offset_align, PAGE_SIZE));
        self.inner.get_mut(&offset_align)
    }

    pub fn dont_use(&self) -> bool {
        self.dont_use
    }

    pub fn sync(&self) {}

    // /// Read the `id` page from the page cache, if not exists, allocate a new
    // /// page and use `read_fn` to read the data into the page.
    // ///
    // /// The return value is determined by the following factors:
    // /// - if has page, return the actual length of the data read (maybe is the
    // ///   buf len or the page rest size)
    // /// - if not has page, first load the miss page (mention that maybe read
    // ///   size can be less than PAGE_SIZE),
    // /// then return the actual length of the data read (maybe is the read_fn's
    // /// return value or the page rest size)
    // async fn read_page<F>(
    //     &mut self,
    //     offset: &mut usize,
    //     buf: &mut [u8],
    //     read_fn: F,
    // ) -> SysResult<usize>
    // where
    //     F: FnOnce(usize, &mut [u8]) -> Pin<Box<dyn Future<Output = SyscallResult>
    // + Send>>, { if *offset > self.end { return Ok(0); } let (page_offset_align,
    //   page_offset) = align_offset(*offset, PAGE_SIZE); let
    //   (end_page_offset_align, end_page_offset) = align_offset(self.end,
    // PAGE_SIZE);

    //     let page_id = *offset >> PAGE_WIDTH;
    //     if let Some(page) = self.inner.get(&page_id) {
    //         let page_len = if page_offset_align == end_page_offset_align {
    //             end_page_offset
    //         } else {
    //             PAGE_SIZE
    //         };
    //         let len = buf.len().min(page_len - page_offset);
    //         unsafe {
    //             core::ptr::copy_nonoverlapping(
    //                 page.as_mut_bytes_array().as_ptr().add(page_offset),
    //                 buf.as_mut_ptr(),
    //                 len,
    //             );
    //         }
    //         *offset += len;
    //         Ok(len)
    //     } else {
    //         let page = Page::new();
    //         // if the len < PAGE_SIZE, means that meet EOF, the last page only
    // [..len] is         // valid !
    //         let _len = read_fn(page_offset_align,
    // page.as_mut_bytes_array()).await?;         self.read_page(offset, buf,
    // read_fn).await     }
    // }

    // pub async fn read<F>(&mut self, offset: usize, mut buf: &mut [u8], read_fn:
    // F) -> SyscallResult where
    //     F: FnOnce(usize, &mut [u8]) -> Pin<Box<dyn Future<Output = SyscallResult>
    // + Send>>, { let mut ret = 0; let mut offset = offset; loop { match
    //   self.read_page(&mut offset, buf, read_fn).await { Ok(0) => break, Ok(n) =>
    //   { ret += n; let tmp = buf; buf = &mut tmp[n..]; } Err(e) => return Err(e),
    //   } } Ok(ret as isize)
    // }
}
