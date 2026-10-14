use std::process::Child;
impl ProcessTree {
    pub fn terminate(&mut self, child: &mut Child) {
        #[cfg(unix)]
        {
            extern "C" {
                fn kill(pid: i32, signal: i32) -> i32;
            }
            unsafe {
                kill(-self.0, 15);
            }
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while std::time::Instant::now() < deadline {
                if child.try_wait().ok().flatten().is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        }
        let _ = child.kill();
        // Drop still enforces worker-tree cleanup after a graceful root exit.
    }
}
// Router workers and MCP subprocesses must stop with their owning server.
#[cfg(windows)]
pub struct ProcessTree(isize);
#[cfg(windows)]
impl ProcessTree {
    pub fn attach(child: &Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        #[repr(C)]
        struct Basic {
            process_time: i64,
            job_time: i64,
            flags: u32,
            min: usize,
            max: usize,
            active: u32,
            affinity: usize,
            priority: u32,
            scheduling: u32,
        }
        #[repr(C)]
        struct Extended {
            basic: Basic,
            io: [u64; 6],
            process_memory: usize,
            job_memory: usize,
            peak_process: usize,
            peak_job: usize,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn CreateJobObjectW(security: *const std::ffi::c_void, name: *const u16) -> isize;
            fn SetInformationJobObject(
                job: isize,
                class: i32,
                data: *const std::ffi::c_void,
                length: u32,
            ) -> i32;
            fn AssignProcessToJobObject(job: isize, process: *mut std::ffi::c_void) -> i32;
        }
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job == 0 {
                return Err("Cannot create server process group.".into());
            }
            let tree = Self(job);
            let mut limits: Extended = std::mem::zeroed();
            limits.basic.flags = 0x2000;
            if SetInformationJobObject(
                job,
                9,
                (&limits as *const Extended).cast(),
                std::mem::size_of::<Extended>() as u32,
            ) == 0
                || AssignProcessToJobObject(job, child.as_raw_handle()) == 0
            {
                return Err("Cannot attach server to its process group.".into());
            }
            Ok(tree)
        }
    }
}
#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        #[link(name = "kernel32")]
        extern "system" {
            fn CloseHandle(handle: isize) -> i32;
        }
        unsafe {
            CloseHandle(self.0);
        }
    }
}
#[cfg(unix)]
pub struct ProcessTree(i32);
#[cfg(unix)]
impl ProcessTree {
    pub fn attach(child: &Child) -> Result<Self, String> {
        Ok(Self(child.id() as i32))
    }
}
#[cfg(unix)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        extern "C" {
            fn kill(pid: i32, signal: i32) -> i32;
        }
        unsafe {
            kill(-self.0, 9);
        }
    }
}
#[cfg(not(any(windows, unix)))]
pub struct ProcessTree;
#[cfg(not(any(windows, unix)))]
impl ProcessTree {
    pub fn attach(_: &Child) -> Result<Self, String> {
        Ok(Self)
    }
}
