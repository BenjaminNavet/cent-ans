//! PB3f (ADR 0091): the planner's read-only work on the performance cores.
//!
//! The factions still play one after the other (spec § 3.4); inside one
//! faction's [`crate::plan_turn`], pure computations on `&CampaignState`
//! (independent planners, recruitment and building options, army anchors)
//! run on a dedicated rayon pool and their results are gathered back in the
//! sequential order: the orders are the same, bit for bit.
//!
//! [`Mode::Sequential`] runs the very same code on the calling thread: the
//! reference of the equality tests.

use std::sync::OnceLock;

use rayon::prelude::*;

/// How the planner runs its independent work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Everything on the calling thread (reference).
    Sequential,
    /// Independent work on [`pool`].
    Parallel,
}

impl Mode {
    /// `(a(), b())`, concurrently in [`Mode::Parallel`].
    pub fn join<A, B, RA, RB>(self, a: A, b: B) -> (RA, RB)
    where
        A: FnOnce() -> RA + Send,
        B: FnOnce() -> RB + Send,
        RA: Send,
        RB: Send,
    {
        match self {
            Mode::Sequential => (a(), b()),
            Mode::Parallel => pool().install(|| rayon::join(a, b)),
        }
    }

    /// `items.iter().map(f).collect()`, in the items' order.
    pub fn map<T, R, F>(self, items: &[T], f: F) -> Vec<R>
    where
        T: Sync,
        R: Send,
        F: Fn(&T) -> R + Sync + Send,
    {
        match self {
            Mode::Sequential => items.iter().map(f).collect(),
            Mode::Parallel => pool().install(|| items.par_iter().map(f).collect()),
        }
    }
}

/// Threads of the pool: the performance cores (`hw.perflevel0.physicalcpu`
/// on Apple silicon: 10 on an M4 Pro), else the available parallelism.
pub fn pool_threads() -> usize {
    performance_cores()
        .or_else(|| std::thread::available_parallelism().ok().map(|n| n.get()))
        .unwrap_or(1)
        .clamp(1, 16)
}

/// The planner's pool, built on first use. Shared by every caller (the
/// turn thread of the bridge, the probes' campaign threads): at most
/// [`pool_threads`] workers whatever the number of concurrent campaigns.
pub fn pool() -> &'static rayon::ThreadPool {
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(pool_threads())
            .thread_name(|i| format!("ai-plan-{i}"))
            .start_handler(|_| raise_thread_priority())
            .build()
            .expect("AI planner thread pool")
    })
}

#[cfg(target_os = "macos")]
fn performance_cores() -> Option<usize> {
    extern "C" {
        fn sysctlbyname(
            name: *const std::ffi::c_char,
            oldp: *mut std::ffi::c_void,
            oldlenp: *mut usize,
            newp: *mut std::ffi::c_void,
            newlen: usize,
        ) -> i32;
    }
    let mut value: i32 = 0;
    let mut len = std::mem::size_of::<i32>();
    // SAFETY: libSystem call with a NUL-terminated name and an output
    // buffer of the announced size; nothing is written on failure.
    let status = unsafe {
        sysctlbyname(
            c"hw.perflevel0.physicalcpu".as_ptr(),
            (&mut value as *mut i32).cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    (status == 0 && value > 0).then_some(value as usize)
}

#[cfg(not(target_os = "macos"))]
fn performance_cores() -> Option<usize> {
    None
}

/// The player waits for the pool (like the turn thread, ADR 0081): on macOS
/// its workers ask for `QOS_CLASS_USER_INITIATED`, else they may land on
/// the efficiency cores.
#[cfg(target_os = "macos")]
fn raise_thread_priority() {
    /// `QOS_CLASS_USER_INITIATED` from `<sys/qos.h>`.
    const QOS_CLASS_USER_INITIATED: u32 = 0x19;
    extern "C" {
        fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
    }
    // SAFETY: libSystem call on the current thread, no pointer involved; a
    // failure only leaves the default quality of service.
    unsafe {
        pthread_set_qos_class_self_np(QOS_CLASS_USER_INITIATED, 0);
    }
}

#[cfg(not(target_os = "macos"))]
fn raise_thread_priority() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_agree_and_keep_the_order() {
        let items: Vec<u32> = (0..1000).collect();
        let square = |x: &u32| u64::from(*x) * u64::from(*x);
        assert_eq!(
            Mode::Sequential.map(&items, square),
            Mode::Parallel.map(&items, square)
        );
        assert_eq!(Mode::Parallel.join(|| 1, || 2), (1, 2));
        assert!(pool_threads() >= 1);
    }
}
