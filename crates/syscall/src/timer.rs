use crate::SyscallResult;

pub trait SyscallTimer {
    /// Retrieves the time of specified clock `clockid`.
    ///
    /// # Error
    /// - `EFAULT`: tp points outside the accessible address space.
    fn clock_gettime(clockid: usize, tp: usize) -> SyscallResult {
        Ok(0)
    }

    /*
        These system calls provide access to interval timers, that is, timers that
        initially expire at some point in the future, and (optionally) at regular
        intervals after that. When a timer expires, a signal is generated for the
        calling process, and the timer is reset to the specified interval (if the
        interval is nonzero).
    */

    /// Places the current value of the timer specified by which in the buffer
    /// pointed to by `curr_value`.
    ///
    /// # Error
    /// - `EFAULT`: `curr_value` is not a valid pointer.
    /// - `EINVAL`: `which` is not one of [`ITimerType`]
    fn getitimer(which: usize, curr_value: usize) -> SyscallResult {
        Ok(0)
    }

    /// Arms or disarms the timer specified by `which`, by setting the timer to
    /// the value specified by `new_value`.
    ///
    /// If old_value is non-NULL, the buffer it points to is used to return the
    /// previous value of the timer (i.e., the same information that is returned
    /// by `getitimer()`).
    ///
    /// If either field in new_value.it_value is nonzero, then the timer is armed
    /// to initially expire at the specified time. If both fields in
    /// `new_value.it_value` are zero, then the timer is disarmed.
    ///
    /// # Error
    /// - `EFAULT`: `new_value` or `old_value` is not a valid pointer.
    /// - `EINVAL`: `which` is not one of [`ITimerType`]
    fn setitimer(which: usize, new_value: usize, old_value: usize) -> SyscallResult {
        Ok(0)
    }

    /// Gets the time as well as the timezone.
    ///
    /// # Error
    /// - `EFAULT`: outside the accessible address
    fn gettimeofday(tv: usize) -> SyscallResult {
        Ok(0)
    }

    /// Suspends the execution of the calling thread until either at least the time specified
    /// in *req has elapsed, or the delivery of a signal that triggers the invocation of a handler
    /// in the calling thread or that terminates the process.
    /// 
    /// # Error
    /// 
    /// - `EFAULT`: Problem with copying information from user space.
    /// - `EINTR`： The pause has been interrupted by a signal that was delivered to the thread (see
    /// signal(7)). The remaining sleep time has been written into *rem so that the thread can easily
    /// call nanosleep() again and continue with the pause.
    /// - `EINVAL`: The value in the tv_nsec field was not in the range 0 to 999999999 or tv_sec was negative.
    fn nanosleep(req: usize, rem: usize) -> SyscallResult {
        Ok(0)
    }

    /// sleep ms millsecond
    fn sleep(ms: usize) -> SyscallResult {
        Ok(0)
    }
}
