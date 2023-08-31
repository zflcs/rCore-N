use alloc::sync::Arc;
use errno::Errno;
use signal_defs::*;
use syscall_interface::{SyscallComm, SyscallResult};

use crate::{arch::mm::VirtAddr, fs::Pipe, read_user, task::cpu, write_user};

use super::SyscallImpl;

impl SyscallComm for SyscallImpl {
    fn pipe(pipefd: *const u32, _flags: usize) -> SyscallResult {
        let curr = cpu().curr.as_ref().unwrap();

        let mut files = curr.files();
        let (pipe_read, pipe_write) = Pipe::new();

        if files.count() + 2 > files.get_limit() {
            return Err(Errno::EMFILE);
        }

        let fd_read = files.push(Arc::new(pipe_read)).unwrap();
        let fd_write = files.push(Arc::new(pipe_write)).unwrap();
        drop(files);

        let fd_data = ((fd_write << 32) | (fd_read & 0xffffffff)) as u64;
        write_user!(curr.mm(), VirtAddr::from(pipefd as usize), fd_data, u64)?;

        Ok(0)
    }

    fn sigaction(signum: usize, act: usize, oldact: usize) -> SyscallResult {
        if !sigvalid(signum) || (act != 0 && sig_kernel_only(signum)) {
            return Err(Errno::EINVAL);
        }

        let curr = cpu().curr.as_ref().unwrap();
        let mut curr_mm = curr.mm();
        let mut sig_actions = curr.sig_actions.lock();

        if oldact != 0 {
            write_user!(curr_mm, oldact.into(), sig_actions[signum - 1], SigAction)?;
        }

        if act != 0 {
            let mut new_act = SigAction::new();
            read_user!(curr_mm, act.into(), new_act, SigAction)?;

            /*
             * POSIX 3.3.1.3:
             *  "Setting a signal action to SIG_IGN for a signal that is
             *   pending shall cause the pending signal to be discarded,
             *   whether or not it is blocked."
             *
             *  "Setting a signal action to SIG_DFL for a signal that is
             *   pending and whose default action is to ignore the signal
             *   (for example, SIGCHLD), shall cause the pending signal to
             *   be discarded, whether or not it is blocked"
             */
            let handler = new_act.handler;
            if handler == SIG_IGN || (handler == SIG_DFL && sig_kernel_ignore(signum)) {
                // TODO!
            }

            let sig_action = &mut sig_actions[signum - 1];
            *sig_action = new_act;
            sig_action
                .mask
                .unset_mask(sigmask(SIGKILL) | sigmask(SIGSTOP));
        }

        Ok(0)
    }

    fn sigprocmask(how: usize, set: usize, oldset: usize, sigsetsize: usize) -> SyscallResult {
        Ok(0)
    }
}
