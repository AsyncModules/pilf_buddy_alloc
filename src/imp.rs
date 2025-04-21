use super::linked_list::LinkedList;

use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::cmp::{max, min};
use core::fmt;
use core::mem::size_of;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

/// A heap that uses buddy system with configurable order.
///
/// # Usage
///
/// Create a heap and add a memory region to it:
/// ```
/// use buddy_system_allocator::*;
/// # use core::mem::size_of;
/// let mut heap = Heap::<32>::empty();
/// # let space: [usize; 100] = [0; 100];
/// # let begin: usize = space.as_ptr() as usize;
/// # let end: usize = begin + 100 * size_of::<usize>();
/// # let size: usize = 100 * size_of::<usize>();
/// unsafe {
///     heap.init(begin, size);
///     // or
///     heap.add_to_heap(begin, end);
/// }
/// ```
pub struct LockFreeHeap<const ORDER: usize> {
    // buddy system with max order of `ORDER`
    // LinkedList已经实现了无锁同步，因此本文件中涉及LinkedList的单个操作同步问题可以不需理会。
    // 但是，多个操作间的数据一致性仍需考虑。
    free_list: [LinkedList; ORDER],

    // statistics
    user: AtomicUsize,
    allocated: AtomicUsize,
    total: AtomicUsize,
}

impl<const ORDER: usize> LockFreeHeap<ORDER> {
    /// Create an empty heap
    pub const fn new() -> Self {
        Self {
            free_list: [LinkedList::EMPTY_LIST; ORDER],
            user: AtomicUsize::new(0),
            allocated: AtomicUsize::new(0),
            total: AtomicUsize::new(0),
        }
    }

    /// Create an empty heap
    pub const fn empty() -> Self {
        Self::new()
    }

    /// Add a range of memory [start, end) to the heap
    pub unsafe fn add_to_heap(&self, mut start: usize, mut end: usize) {
        // avoid unaligned access on some platforms
        start = (start + size_of::<usize>() - 1) & (!size_of::<usize>() + 1);
        end &= !size_of::<usize>() + 1;
        assert!(start <= end);

        let mut total = 0;
        let mut current_start = start;

        while current_start + size_of::<usize>() <= end {
            let lowbit = current_start & (!current_start + 1);
            let mut size = min(lowbit, prev_power_of_two(end - current_start));

            // If the order of size is larger than the max order,
            // split it into smaller blocks.
            let mut order = size.trailing_zeros() as usize;
            if order > ORDER - 1 {
                order = ORDER - 1;
                size = 1 << order;
            }
            total += size;

            self.free_list[order].push(current_start as *mut _); // 写
            current_start += size;
        }

        self.total.fetch_add(total, Ordering::Relaxed); // 写
    }

    /// Add a range of memory [start, start+size) to the heap
    pub unsafe fn init(&mut self, start: usize, size: usize) {
        self.add_to_heap(start, start + size);
    }

    /// Alloc a range of memory from the heap satifying `layout` requirements
    /// 返回值是偏移量
    pub fn alloc_(&self, layout: Layout) -> Result<NonNull<u8>, ()> {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let class = size.trailing_zeros() as usize;
        let mut current_block;
        for i in class..self.free_list.len() {
            if !self.free_list[i].is_empty() {
                // 先尝试从列表中取出一个块，如果当前的这个链表在判断非空之后无法取出块，则跳过后续的流程，尝试从下一个链表中取出空闲块
                current_block = self.free_list[i].pop();
                if current_block.is_none() {
                    // 这里直接使用 continue 会导致在分配时出现大空闲块未切分完成，取不到空闲块的情况
                    // 这里需要限制 ORDER 的大小，ORDER 的大小决定了分配和合并时占用的时间
                    // ORDER 小一点，可以快速结束切分以及合并的过程，从而出现这个问题的概率
                    continue;
                }
                assert!(current_block.is_some());
                // 判断块是否需要切分，若 i == class，则不需要进行切分
                if i != class {
                    for j in (class + 1..i + 1).rev() {
                        let block = current_block.unwrap() as usize;
                        // 将分裂后的块插入 free_list[j-1]
                        unsafe { self.free_list[j - 1].push((block + (1 << (j - 1))) as *mut _) };
                        current_block = Some(block as _);
                    }
                }
                // 执行到这里时，说明已经分配成功了
                let result = NonNull::new(current_block.unwrap() as *mut u8).unwrap();
                self.user.fetch_add(layout.size(), Ordering::Relaxed); // 写user
                self.allocated.fetch_add(size, Ordering::Relaxed); // 写allocater
                return Ok(result);
            }
        }
        Err(())
    }

    /// Dealloc a range of memory from the heap
    /// ptr 参数为偏移量
    /// 这个函数的写操作太多了，不好同步。看看能否减少，比如先插入再合并改为先合并再插入。
    pub fn dealloc_(&self, ptr: NonNull<u8>, layout: Layout) {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let class = size.trailing_zeros() as usize;

        unsafe {
            // 合并空闲块
            let mut current_ptr = ptr.as_ptr() as usize;
            let mut current_class = class;

            while current_class < self.free_list.len() - 1 {
                let buddy = current_ptr ^ (1 << current_class);
                // 返回 true，当前级别的空闲链表中存在可以合并的节点且已经被删除，可以直接合并
                if self.free_list[current_class].delete(buddy as _) {
                    current_ptr = min(current_ptr, buddy);
                    current_class += 1;
                } else {
                    // 没有可以合并的块，插入到当前的空闲链表中
                    self.free_list[current_class].push(current_ptr as *mut _); // 写free_list[current_class]
                    break;
                }
            }

            // 此时合并的块无法在循环中 push 回链表，因此在此处push
            if current_class == self.free_list.len() - 1 {
                self.free_list[current_class].push(current_ptr as *mut _); // 写free_list[current_class]
            }
        }

        self.user.fetch_sub(layout.size(), Ordering::Relaxed); // 写user
        self.allocated.fetch_sub(size, Ordering::Relaxed); // 写allocater
    }

    /// Return the number of bytes that user requests
    pub fn stats_alloc_user(&self) -> usize {
        self.user.load(Ordering::Relaxed)
    }

    /// Return the number of bytes that are actually allocated
    pub fn stats_alloc_actual(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }

    /// Return the total number of bytes in the heap
    pub fn stats_total_bytes(&self) -> usize {
        self.total.load(Ordering::Relaxed)
    }
}

impl<const ORDER: usize> fmt::Debug for LockFreeHeap<ORDER> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("LockFreeHeap")
            .field("user", &self.user)
            .field("allocated", &self.allocated)
            .field("total", &self.total)
            .finish()
    }
}

unsafe impl<const ORDER: usize> GlobalAlloc for LockFreeHeap<ORDER> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_(layout)
            .ok()
            .map_or(core::ptr::null_mut(), |allocation| allocation.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.dealloc_(NonNull::new_unchecked(ptr), layout);
    }
}

pub(crate) fn prev_power_of_two(num: usize) -> usize {
    1 << (usize::BITS as usize - num.leading_zeros() as usize - 1)
}
