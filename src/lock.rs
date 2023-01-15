use std::marker::{PhantomData};
use std::sync::atomic::{AtomicBool, Ordering};

/// `VJoyLock` is used to ensure that all vJoy access is contained to a single thread at any given
/// point in time, while introducing minimal overhead.
///
/// While the documentation for the vJoyInterface does not explicitly mention any notes about
/// thread-safety, its code appears to make use of static mutable data in a number of places. Thus,
/// all access to vJoy must be constrained to a single thread at any one time.
///
/// Access to a `VJoyLock` ensures that all vJoy access occurs on the same thread. This occurs by
/// ensuring that a `VJoyLock` can only be created if none others exist (tracked via the AtomicBool
/// `LOCKED`) or is cloned from an existing `VJoyLock`. Since `VJoyLock` is neither Send nor Sync,
/// this means that all `VJoyLock` objects that exist at any given time must exist on the same
/// thread. Thus, by ensuring that all vJoy access occurs in the presence of a `VJoyLock`, all vJoy
/// access is limited to a single thread.
///
/// A reference count is maintained to keep track of the number of existing `VJoyLock` objects.
/// Since all `VJoyLock` objects exist on the same thread, this can safely be stored in a `static
/// mut`, allowing `VJoyLock` to be a zero-sized struct, while also preventing the need for the
/// overhead of atomic reference counting. This should be sound as long as that reference count is
/// only ever accessed in the presence of a `VJoyLock` object.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VJoyLock(PhantomData<*const ()>);

// VJoyLock must not be Send or Sync, as its purpose is to contain vJoy access to a single thread.
static_assertions::assert_not_impl_any!(VJoyLock: Send, Sync);

impl VJoyLock {
    pub fn new() -> Option<VJoyLock> {
        if LOCKED.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            // UNSAFE: Because LOCKED was just changed from false to true, there must not be any
            // existing VJoyLock objects; since REFS is only accessed by VJoyLock objects, this
            // means that accessing REFS is safe.
            unsafe {
                debug_assert_eq!(REFS, 0);
                REFS = 1;
            }

            Some(VJoyLock(PhantomData))
        } else {
            None
        }
    }
}

impl Clone for VJoyLock {
    fn clone(&self) -> Self {
        // UNSAFE: This is in the presence of a VJoyLock (self), so accessing REFS is safe.
        unsafe {
            REFS += 1;
        }

        VJoyLock(PhantomData)
    }
}

impl Drop for VJoyLock {
    fn drop(&mut self) {
        // UNSAFE: This is in the presence of a VJoyLock (self), so accessing REFS is safe.
        let unlock = unsafe {
            debug_assert!(REFS > 0);
            REFS -= 1;

            // If REFS was set to 0, then self is the last existing VJoyLock object, and LOCKED may
            // now be set back to false so that new VJoyLocks may be created.
            REFS == 0
        };

        if unlock {
            LOCKED.store(false, Ordering::Release);
        }
    }
}

static LOCKED: AtomicBool = AtomicBool::new(false);
static mut REFS: usize = 0;
