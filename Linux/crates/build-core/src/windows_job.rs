//! Own a Windows invocation from before its first instruction until every
//! descendant stops. The non-inherited job handle also closes on app exit.

use std::io;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Duration;
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
        },
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
            JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
            TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME},
    },
};

// The owned integer makes sharing the job across the runner's wait/cancel
// threads explicit; only Drop closes it, and the handle is never inherited.
pub(crate) struct WindowsJob(usize);

impl WindowsJob {
    fn handle(&self) -> HANDLE {
        self.0 as HANDLE
    }

    pub(crate) fn spawn(command: &mut Command) -> io::Result<(Child, Arc<Self>)> {
        let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if raw.is_null() {
            return Err(io::Error::last_os_error());
        }
        let job = Arc::new(Self(raw as usize));
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe {
            SetInformationJobObject(
                raw,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }

        // std preserves native Unicode, environment and pipe handling. It
        // releases the primary-thread handle, so find that suspended thread
        // by PID and resume it only after the job owns the process.
        command.creation_flags(super::CREATE_NO_WINDOW | 0x0000_0004); // CREATE_SUSPENDED
        let mut child = command.spawn()?;
        let attached = unsafe { AssignProcessToJobObject(raw, child.as_raw_handle()) } != 0;
        let ready = if attached {
            resume(&child)
        } else {
            Err(io::Error::last_os_error())
        };
        if let Err(error) = ready {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok((child, job))
    }

    /// Killing the job schedules all descendants for termination. Wait for
    /// the accounting count to reach zero before consuming output/cleaning
    /// files; a descendant may have redirected its inherited stdout.
    pub(crate) fn stop(&self) -> io::Result<()> {
        if unsafe { TerminateJobObject(self.handle(), 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        loop {
            let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
            if unsafe {
                QueryInformationJobObject(
                    self.handle(),
                    JobObjectBasicAccountingInformation,
                    (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                    std::mem::size_of_val(&accounting) as u32,
                    std::ptr::null_mut(),
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            if accounting.ActiveProcesses == 0 {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for WindowsJob {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle());
        }
    }
}

fn resume(child: &Child) -> io::Result<()> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    struct Snapshot(HANDLE);
    impl Drop for Snapshot {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    let _snapshot = Snapshot(snapshot);
    let mut entry = THREADENTRY32::default();
    entry.dwSize = std::mem::size_of_val(&entry) as u32;
    let mut found = unsafe { Thread32First(snapshot, &mut entry) } != 0;
    while found {
        if entry.th32OwnerProcessID == child.id() {
            let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            if thread.is_null() {
                return Err(io::Error::last_os_error());
            }
            let resumed = unsafe { ResumeThread(thread) };
            let result = if resumed == u32::MAX {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            };
            unsafe {
                CloseHandle(thread);
            }
            return result;
        }
        found = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Suspended process thread not found",
    ))
}
