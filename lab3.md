### 实现的功能
有关上一章内容的迁移，基本上无需变动。

`sys_spawn()`的实现基本直接copy`TaskControlBlock::new()`的实现即可，增加的地方是设置好父子进程关系。

有关stride算法的实现，为TCB增加`stride`，`pass`字段，并在调度时进行相应的更新即可。具体来说，是在`suspend_current_and_run_next()`中更新当前进程的`pass`，并在`fetch_task()`中选择`pass`最小的进程。不用考虑`exit_current_and_run_next()`，因为进程不会再被调度了。

### 问题回答
1. 不是。p2调度结束后，`p2.stride`会因溢出变为4，小于`p1.stride`。因此被调度者仍是p2。
2. 进程优先级大于2，就可以保证每次步进时，stride的增量小或等于128，两个stride的差值的绝对值也就不会超过128。模256的环形空间上，我们保证了他们只有一条路径连接（短路径）。接着，就可以通过直接比较的方式判断谁的stride更小。
3.  
```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let bignum_half = 128;
        let diff: i64 = self.0 as i64 - other.0 as i64;
        let norm_diff = if diff > bignum_half {
            diff - 256
        } else if diff < -bignum_half {
            diff + 256
        } else {
            diff
        };
        Some(norm_diff.cmp(&0))
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```

### 附
`task::mod.c`中`IDLE_PID`的定义疑似存在问题，应该是`INIT_PID`？Idle并非一个进程，也不占用相关资源，仅仅是一个调度上下文。

### 荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
   
    null

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容:
    
    null

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。