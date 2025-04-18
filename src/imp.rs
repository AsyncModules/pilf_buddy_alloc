use super::linked_list::LinkedList;

extern crate spin;

use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::cmp::{max, min};
use core::fmt;
use core::mem::size_of;
use core::ops::Deref;
use core::ptr::NonNull;
use spin::Mutex;

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
pub struct Heap<const ORDER: usize> {
    // buddy system with max order of `ORDER`
    // LinkedList已经实现了无锁同步，因此本文件中涉及LinkedList的单个操作同步问题可以不需理会。
    // 但是，多个操作间的数据一致性仍需考虑。
    free_list: [LinkedList; ORDER],

    // statistics
    user: usize,
    allocated: usize,
    total: usize,
}

impl<const ORDER: usize> Heap<ORDER> {
    /// Create an empty heap
    pub const fn new() -> Self {
        Heap {
            free_list: [LinkedList::new(); ORDER],
            user: 0,
            allocated: 0,
            total: 0,
        }
    }

    /// Create an empty heap
    pub const fn empty() -> Self {
        Self::new()
    }

    /// Add a range of memory [start, end) to the heap
    pub unsafe fn add_to_heap(&mut self, mut start: usize, mut end: usize) {
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

            self.free_list[order].push(current_start as *mut usize); // 写
            current_start += size;
        }

        self.total += total; // 写
    }

    /// Add a range of memory [start, start+size) to the heap
    pub unsafe fn init(&mut self, start: usize, size: usize) {
        self.add_to_heap(start, start + size);
    }

    /// Alloc a range of memory from the heap satifying `layout` requirements
    /// 返回值是偏移量
    pub fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, ()> {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let class = size.trailing_zeros() as usize;
        for i in class..self.free_list.len() { // 这句的len是常数，不涉及同步问题
            // Find the first non-empty size class
            if !self.free_list[i].is_empty() {  // 读free_list[i]
                let mut current_block: Option<*mut usize> = None;
                // 判断块是否需要切分
                if i == class {
                    current_block = self.free_list[i].pop(); // 此处使用了之前获取的非空条件，而该条件可能失效。因此需要增加失败重试。
                    assert!(current_block.is_some());
                }
                else {
                    // Split buffers
                    for j in (class + 1..i + 1).rev() {
                        if let Some(block) = current_block.or_else(|| {
                            self.free_list[j].pop() // 写free_list[i]，只会在循环第一次执行。此处使用了之前获取的非空条件，而该条件可能失效。因此需要增加失败重试。
                        }) {
                            // 这里得到的 block 是偏移量，freelist push 的参数也是偏移量，因此不用进行修改
                            unsafe {
                                self.free_list[j - 1]
                                    .push((block as usize + (1 << (j - 1))) as *mut usize); // 写free_list[j-1]
                            }
                            current_block = Some(block);
                        } else {
                            return Err(());
                        }

                        // if let Some(block) = self.free_list[j].pop() {  // 写free_list[j]
                        //     // 这里得到的 block 是偏移量，freelist push 的参数也是偏移量，因此不用进行修改
                        //     unsafe {
                        //         self.free_list[j - 1]
                        //             .push((block as usize + (1 << (j - 1))) as *mut usize);
                        //         self.free_list[j - 1].push(block); // 写free_list[j-1]
                        //     }
                        // } else {
                        //     return Err(());
                        // }
                    }
                }

                let result = NonNull::new(current_block.unwrap() as *mut u8);
                // let result = NonNull::new(
                //     self.free_list[class]
                //         .pop()
                //         .expect("current block should have free space now")
                //         as *mut u8,
                // ); // 写free_list[class]
                if let Some(result) = result {
                    self.user += layout.size(); // 写user
                    self.allocated += size; // 写allocater
                    return Ok(result);
                } else {
                    return Err(());
                }
            }
        }
        Err(())
    }

    /// Dealloc a range of memory from the heap
    /// ptr 参数为偏移量
    /// 这个函数的写操作太多了，不好同步。看看能否减少，比如先插入再合并改为先合并再插入。
    pub fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let class = size.trailing_zeros() as usize;

        unsafe {
            // // Put back into free list
            // self.free_list[class].push(ptr.as_ptr() as *mut usize); // 写free_list[class]

            // Merge free buddy lists
            let mut current_ptr = ptr.as_ptr() as usize;
            let mut current_class = class;

            while current_class < self.free_list.len() - 1 { // 不涉及同步
                let buddy = current_ptr ^ (1 << current_class);
                let mut flag = false;
                for block in self.free_list[current_class].iter_mut() {
                    if block.value() as usize == buddy { // 读ListNode
                        block.pop(); // 写ListNode
                        flag = true;
                        break;
                    }
                }

                // Free buddy found
                if flag {
                    // self.free_list[current_class].pop(); // 写free_list[current_class]
                    current_ptr = min(current_ptr, buddy);
                    current_class += 1;
                    // self.free_list[current_class].push(current_ptr as *mut usize); // 写free_list[current_class]
                } else {
                    self.free_list[current_class].push(current_ptr as *mut usize); // 写free_list[current_class]
                    break;
                }
            }

            if current_class == self.free_list.len() - 1 {
                // 此时合并的块无法在循环中push回链表，因此在此处push。
                self.free_list[current_class].push(current_ptr as *mut usize); // 写free_list[current_class]
            }
        }

        self.user -= layout.size(); // 写user
        self.allocated -= size; // 写allocater
    }

    /// Return the number of bytes that user requests
    pub fn stats_alloc_user(&self) -> usize {
        self.user
    }

    /// Return the number of bytes that are actually allocated
    pub fn stats_alloc_actual(&self) -> usize {
        self.allocated
    }

    /// Return the total number of bytes in the heap
    pub fn stats_total_bytes(&self) -> usize {
        self.total
    }
}

impl<const ORDER: usize> fmt::Debug for Heap<ORDER> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Heap")
            .field("user", &self.user)
            .field("allocated", &self.allocated)
            .field("total", &self.total)
            .finish()
    }
}

/// A locked version of `Heap`
///
/// # Usage
///
/// Create a locked heap and add a memory region to it:
/// ```
/// use buddy_system_allocator::*;
/// # use core::mem::size_of;
/// let mut heap = LockedHeap::<32>::new();
/// # let space: [usize; 100] = [0; 100];
/// # let begin: usize = space.as_ptr() as usize;
/// # let end: usize = begin + 100 * size_of::<usize>();
/// # let size: usize = 100 * size_of::<usize>();
/// unsafe {
///     heap.lock().init(begin, size);
///     // or
///     heap.lock().add_to_heap(begin, end);
/// }
/// ```
pub struct LockedHeap<const ORDER: usize>(Mutex<Heap<ORDER>>);

impl<const ORDER: usize> LockedHeap<ORDER> {
    /// Creates an empty heap
    pub const fn new() -> Self {
        LockedHeap(Mutex::new(Heap::<ORDER>::new()))
    }

    /// Creates an empty heap
    pub const fn empty() -> Self {
        LockedHeap(Mutex::new(Heap::<ORDER>::new()))
    }
}

impl<const ORDER: usize> Deref for LockedHeap<ORDER> {
    type Target = Mutex<Heap<ORDER>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl<const ORDER: usize> GlobalAlloc for LockedHeap<ORDER> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0
            .lock()
            .alloc(layout)
            .ok()
            .map_or(core::ptr::null_mut(), |allocation| allocation.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.lock().dealloc(NonNull::new_unchecked(ptr), layout)
    }
}

/// A locked version of `Heap` with rescue before oom
///
/// # Usage
///
/// Create a locked heap:
/// ```
/// use buddy_system_allocator::*;
/// let heap = LockedHeapWithRescue::new(|heap: &mut Heap<32>, layout: &core::alloc::Layout| {});
/// ```
///
/// Before oom, the allocator will try to call rescue function and try for one more time.
pub struct LockedHeapWithRescue<const ORDER: usize> {
    inner: Mutex<Heap<ORDER>>,
    rescue: fn(&mut Heap<ORDER>, &Layout),
}

impl<const ORDER: usize> LockedHeapWithRescue<ORDER> {
    /// Creates an empty heap
    pub const fn new(rescue: fn(&mut Heap<ORDER>, &Layout)) -> Self {
        LockedHeapWithRescue {
            inner: Mutex::new(Heap::<ORDER>::new()),
            rescue,
        }
    }
}

impl<const ORDER: usize> Deref for LockedHeapWithRescue<ORDER> {
    type Target = Mutex<Heap<ORDER>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

unsafe impl<const ORDER: usize> GlobalAlloc for LockedHeapWithRescue<ORDER> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut inner = self.inner.lock();
        match inner.alloc(layout) {
            Ok(allocation) => allocation.as_ptr(),
            Err(_) => {
                (self.rescue)(&mut inner, &layout);
                inner
                    .alloc(layout)
                    .ok()
                    .map_or(core::ptr::null_mut(), |allocation| allocation.as_ptr())
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner
            .lock()
            .dealloc(NonNull::new_unchecked(ptr), layout)
    }
}

pub(crate) fn prev_power_of_two(num: usize) -> usize {
    1 << (usize::BITS as usize - num.leading_zeros() as usize - 1)
}
