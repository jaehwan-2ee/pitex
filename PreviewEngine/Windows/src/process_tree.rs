//! Keep each compiler invocation and all of its children owned by the helper.
use std::io;
use std::process::{Child, Command};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
        JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
        TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    },
};

#[cfg(windows)]
fn resume(child: &Child) -> io::Result<()> {
    use windows_sys::Win32::{
        Foundation::INVALID_HANDLE_VALUE,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD,
                THREADENTRY32,
            },
            Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME},
        },
    };
    // std::process retains only the process handle. Enumerating the primary
    // thread lets us use its native Unicode/environment/stdio setup while
    // still assigning the process to its job BEFORE it executes any code.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let mut entry = THREADENTRY32::default();
    entry.dwSize = std::mem::size_of_val(&entry) as u32;
    let mut found = unsafe { Thread32First(snapshot, &mut entry) } != 0;
    let mut result = Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Suspended compiler thread not found",
    ));
    while found {
        if entry.th32OwnerProcessID == child.id() {
            let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            result = if thread.is_null() {
                Err(io::Error::last_os_error())
            } else {
                let resumed = unsafe { ResumeThread(thread) };
                let result = if resumed == u32::MAX {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(())
                };
                unsafe { CloseHandle(thread) };
                result
            };
            break;
        }
        found = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snapshot) };
    result
}

pub struct ProcessTree {
    pub child: Child,
    #[cfg(windows)]
    job: HANDLE,
}

impl ProcessTree {
    pub fn spawn(command: &mut Command) -> io::Result<Self> {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use std::os::windows::process::CommandExt;
            // A job owns compiler descendants (xdvipdfmx, package manager, etc.), and
            // its non-inherited handle closes even if the helper crashes.
            let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if job.is_null() {
                return Err(io::Error::last_os_error());
            }
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if unsafe {
                SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    std::mem::size_of_val(&limits) as u32,
                )
            } == 0
            {
                let error = io::Error::last_os_error();
                unsafe { CloseHandle(job) };
                return Err(error);
            }
            command.creation_flags(0x0800_0004); // CREATE_NO_WINDOW | CREATE_SUSPENDED
            let mut child = match command.spawn() {
                Ok(child) => child,
                Err(error) => {
                    unsafe { CloseHandle(job) };
                    return Err(error);
                }
            };
            let assigned = unsafe { AssignProcessToJobObject(job, child.as_raw_handle()) } != 0;
            let resumed = if assigned {
                resume(&child)
            } else {
                Err(io::Error::last_os_error())
            };
            if let Err(error) = resumed {
                let _ = child.kill();
                let _ = child.wait();
                unsafe { CloseHandle(job) };
                return Err(error);
            }
            Ok(Self { child, job })
        }
        #[cfg(not(windows))]
        {
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                command.process_group(0);
            }
            Ok(Self {
                child: command.spawn()?,
            })
        }
    }

    pub fn stop(&mut self) {
        #[cfg(windows)]
        if !self.job.is_null() {
            unsafe { TerminateJobObject(self.job, 1) };
        }
        #[cfg(unix)]
        {
            // Used by the portable backend's Linux regression tests too.
            unsafe extern "C" {
                fn kill(pid: i32, signal: i32) -> i32;
            }
            unsafe { kill(-(self.child.id() as i32), 9) };
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        #[cfg(windows)]
        if !self.job.is_null() {
            // TerminateJobObject schedules termination asynchronously. Wait
            // for descendants too before the next generation changes source
            // or output files (a closing xdvipdfmx may still hold them open).
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            loop {
                let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
                let read = unsafe {
                    QueryInformationJobObject(
                        self.job,
                        JobObjectBasicAccountingInformation,
                        (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                        std::mem::size_of_val(&accounting) as u32,
                        std::ptr::null_mut(),
                    )
                } != 0;
                if !read || accounting.ActiveProcesses == 0 || std::time::Instant::now() >= deadline
                {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

impl Drop for ProcessTree {
    fn drop(&mut self) {
        self.stop();
        #[cfg(windows)]
        if !self.job.is_null() {
            unsafe { CloseHandle(self.job) };
        }
    }
}
