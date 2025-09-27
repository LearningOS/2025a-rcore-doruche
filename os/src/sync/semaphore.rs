//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_set::BTreeSet;
use alloc::vec::Vec;
use alloc::{collections::VecDeque, sync::Arc};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                let tid = task.inner_exclusive_access().res.as_ref().unwrap().tid;
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self) {
        trace!("kernel: Semaphore::down");
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            let task = current_task().unwrap();
            inner.wait_queue.push_back(task);
            drop(inner);
            block_current_and_run_next();
        }
    }

    pub fn up_with_detection(&self, detector: &mut DeadlockDetector, sem_id: usize) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;

        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;

        detector.dealloc_for(tid, sem_id);
        detector.add_avail(sem_id, 1);
       
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                wakeup_task(task);
            }
        }
    }

    pub fn down_with_detection(&self, detector: &mut DeadlockDetector, sem_id: usize) -> isize {
        trace!("kernel: Semaphore::down");
        let mut inner = self.inner.exclusive_access();
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        
        detector.do_req(tid, sem_id);

        if detector.detect_deadlock() {
            // deadlock detected
            detector.finish_req(tid);
            return -0xdead;
        }

        inner.count -= 1;

        if inner.count < 0 {
            let task = current_task().unwrap();
            inner.wait_queue.push_back(task);
            drop(inner);
            return -1; // tell caller to block_current_and_run_next();
            // block_current_and_run_next();
        } else {
            detector.finish_req(tid);
            detector.alloc_for(tid, sem_id);
            detector.add_avail(sem_id, -1);
        }

        0
    }
}

pub struct DeadlockDetector {
    pub avail: BTreeMap<usize, isize>, // semaphore_id -> available count
    pub alloc: BTreeMap<usize, BTreeMap<usize, isize>>, // thread_id -> (semaphore_id -> allocated count ( P - V ))
    pub req: BTreeMap<usize, usize>, // thread_id -> requested semaphore_id
}

impl DeadlockDetector {
    pub fn new() -> Self {
        Self {
            avail: BTreeMap::new(),
            alloc: BTreeMap::new(),
            req: BTreeMap::new(),
        }
    }

    pub fn add_avail(&mut self, sem_id: usize, count: isize) {
        self.avail.entry(sem_id)
            .and_modify(|c| *c += count)
            .or_insert(count);
    }

    pub fn alloc_for(&mut self, tid: usize, sem_id: usize) {
        self.alloc.entry(tid)
            .and_modify(|m| {
                m.entry(sem_id).and_modify(|c| *c += 1)
                    .or_insert(1);
            })
            .or_insert_with(|| {
                let mut m = BTreeMap::new();
                m.insert(sem_id, 1);
                m
            });
    }

    pub fn dealloc_for(&mut self, tid: usize, sem_id: usize) {
        self.alloc.entry(tid)
            .and_modify(|m| {
                m.entry(sem_id).and_modify(|c| {
                    *c -= 1;
                }).or_insert(-1);
            })
            .or_insert_with(|| {
                let mut m = BTreeMap::new();
                m.insert(sem_id, -1);
                m
            });
    }

    pub fn do_req(&mut self, tid: usize, sem_id: usize) {
        assert!(self.req.insert(tid, sem_id).is_none());
    }

    pub fn finish_req(&mut self, tid: usize) {
        assert!(self.req.remove(&tid).is_some());
    }

    pub fn detect_deadlock(&self) -> bool {
        let mut avail = self.avail.clone();
        let mut alloc = self.alloc.clone();
        let mut req = self.req.clone();
        let no_req: Vec<usize> = alloc.keys().filter(|k| !req.contains_key(k)).map(|k| *k).collect();
        println!(">>> DeadlockDetector state:");
        println!(">>>   avail: {:#?}", avail);
        println!(">>>   alloc: {:#?}", alloc);
        println!(">>>   req: {:#?}", req);
        println!(">>>   no_req: {:#?}", no_req);
        let mut changed = true;

        // clear allocations of threads that do not have any requests
        for tid in no_req {
            if let Some(m) = alloc.remove(&tid) {
                for (s_id, count) in m {
                    avail.entry(s_id)
                        .and_modify(|v| *v += count)
                        .or_insert(count);
                }
            }
        }

        while changed {
            changed = false;
            let reqs: Vec<(usize, usize)> = req.iter().map(|(k, v)| (*k, *v)).collect();
            for (tid, sem_id) in reqs {
                if let Some(c) = avail.get_mut(&sem_id) {
                    if *c > 0 {
                        // can satisfy this request
                        // *c -= 1;
                        if let Some(m) = alloc.remove(&tid) {
                            for (s_id, count) in m {
                                avail.entry(s_id)
                                    .and_modify(|v| *v += count)
                                    .or_insert(count);
                            }
                        }
                        req.remove(&tid);
                        changed = true;
                    }
                }
            }
        }

        !req.is_empty() // if there are still requests that cannot be satisfied, deadlock exists
    }
}