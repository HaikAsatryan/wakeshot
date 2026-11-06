//! One child in a pseudo console, inside a job object that holds its whole tree, so the driver can end exactly
//! the processes it started and nothing else.

use std::ffi::{OsStr, OsString, c_void};
use std::os::windows::ffi::OsStrExt;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows::Win32::System::Console::{COORD, ClosePseudoConsole, CreatePseudoConsole, HPCON};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectBasicAccountingInformation, JobObjectBasicProcessIdList,
    JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
    TerminateJobObject,
};
use windows::Win32::System::Pipes::CreatePipe;
use windows::Win32::System::Threading::{
    CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
    EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, InitializeProcThreadAttributeList,
    LPPROC_THREAD_ATTRIBUTE_LIST, OpenProcess, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
    PROCESS_INFORMATION, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    QueryFullProcessImageNameW, ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOEXW, STARTUPINFOW,
    TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
};
use windows::core::{PCWSTR, PWSTR};

pub struct Child {
    pcon: HPCON,
    input: HANDLE,
    process: HANDLE,
    job: HANDLE,
    pub pid: u32,
    reader: Option<JoinHandle<()>>,
}

/// Raw handle value moved into the reader thread; `HANDLE` itself is not `Send`.
struct SendHandle(isize);
unsafe impl Send for SendHandle {}

fn wide(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain(Some(0)).collect()
}

/// `NAME=value` pairs sorted by name without case, each ended by a NUL, the block ended by one more.
pub fn env_block(vars: &[(OsString, OsString)]) -> Vec<u16> {
    let mut sorted: Vec<_> = vars.iter().collect();
    sorted.sort_by_key(|(name, _)| name.to_string_lossy().to_uppercase());
    let mut block = Vec::new();
    for (name, value) in sorted {
        block.extend(name.encode_wide());
        block.push(u16::from(b'='));
        block.extend(value.encode_wide());
        block.push(0);
    }
    if block.is_empty() {
        block.push(0);
    }
    block.push(0);
    block
}

impl Child {
    pub fn spawn(
        command_line: &str,
        cwd: Option<&str>,
        vars: &[(OsString, OsString)],
        cols: u16,
        rows: u16,
        output: Sender<Vec<u8>>,
    ) -> Result<Self, String> {
        unsafe { Self::spawn_inner(command_line, cwd, vars, cols, rows, output) }
    }

    unsafe fn spawn_inner(
        command_line: &str,
        cwd: Option<&str>,
        vars: &[(OsString, OsString)],
        cols: u16,
        rows: u16,
        output: Sender<Vec<u8>>,
    ) -> Result<Self, String> {
        let err = |what: &str, e: windows::core::Error| format!("{what}: {e}");
        let (mut in_read, mut in_write) = (HANDLE::default(), HANDLE::default());
        let (mut out_read, mut out_write) = (HANDLE::default(), HANDLE::default());
        unsafe {
            CreatePipe(&mut in_read, &mut in_write, None, 0).map_err(|e| err("input pipe", e))?;
            CreatePipe(&mut out_read, &mut out_write, None, 0)
                .map_err(|e| err("output pipe", e))?;
            let size = COORD {
                X: cols as i16,
                Y: rows as i16,
            };
            let pcon = CreatePseudoConsole(size, in_read, out_write, 0)
                .map_err(|e| err("CreatePseudoConsole", e))?;
            // The pseudo console holds its own copies; ours would keep the pipes open after it closes.
            let _ = CloseHandle(in_read);
            let _ = CloseHandle(out_write);

            let reader_handle = SendHandle(out_read.0 as isize);
            let reader = std::thread::spawn(move || {
                let handle = HANDLE(reader_handle.0 as *mut c_void);
                let mut buf = vec![0u8; 65536];
                loop {
                    let mut got = 0u32;
                    if ReadFile(handle, Some(&mut buf), Some(&mut got), None).is_err() || got == 0 {
                        break;
                    }
                    if output.send(buf[..got as usize].to_vec()).is_err() {
                        break;
                    }
                }
                let _ = CloseHandle(handle);
            });

            let mut list_size = 0usize;
            let _ = InitializeProcThreadAttributeList(None, 1, None, &mut list_size);
            let mut list_buf = vec![0u8; list_size];
            let list = LPPROC_THREAD_ATTRIBUTE_LIST(list_buf.as_mut_ptr().cast());
            InitializeProcThreadAttributeList(Some(list), 1, None, &mut list_size)
                .map_err(|e| err("attribute list", e))?;
            let attached = UpdateProcThreadAttribute(
                list,
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                Some(pcon.0 as *const c_void),
                size_of::<HPCON>(),
                None,
                None,
            );
            let mut info = STARTUPINFOEXW::default();
            info.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
            // Null standard handles, as Windows Terminal passes: the child must not get the driver's own (a log
            // file when started from a script), only the pseudo console's.
            info.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
            info.lpAttributeList = list;

            let mut cmd: Vec<u16> = wide(OsStr::new(command_line));
            let cwd_w = cwd.map(|d| wide(OsStr::new(d)));
            let cwd_p = cwd_w
                .as_ref()
                .map_or(PCWSTR::null(), |w| PCWSTR(w.as_ptr()));
            let env = env_block(vars);
            let mut pi = PROCESS_INFORMATION::default();
            let created = attached.and_then(|()| {
                create_process(
                    PWSTR(cmd.as_mut_ptr()),
                    env.as_ptr().cast(),
                    cwd_p,
                    &info.StartupInfo,
                    &mut pi,
                )
            });
            DeleteProcThreadAttributeList(list);
            drop(list_buf);
            if let Err(e) = created {
                ClosePseudoConsole(pcon);
                let _ = CloseHandle(in_write);
                let _ = reader.join();
                return Err(err("CreateProcessW", e));
            }

            // Into the job before it runs a single instruction, so not even its first grandchild can escape.
            let job =
                CreateJobObjectW(None, PCWSTR::null()).map_err(|e| err("CreateJobObjectW", e));
            let job = job.and_then(|job| {
                let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    (&raw const limits).cast(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
                .and_then(|()| AssignProcessToJobObject(job, pi.hProcess))
                .map(|()| job)
                .map_err(|e| err("job", e))
            });
            let job = match job {
                Ok(job) => job,
                Err(e) => {
                    let _ = TerminateProcess(pi.hProcess, 1);
                    let _ = CloseHandle(pi.hThread);
                    let _ = CloseHandle(pi.hProcess);
                    ClosePseudoConsole(pcon);
                    let _ = CloseHandle(in_write);
                    let _ = reader.join();
                    return Err(e);
                }
            };
            ResumeThread(pi.hThread);
            let _ = CloseHandle(pi.hThread);
            Ok(Self {
                pcon,
                input: in_write,
                process: pi.hProcess,
                job,
                pid: pi.dwProcessId,
                reader: Some(reader),
            })
        }
    }

    pub fn write(&self, mut bytes: &[u8]) -> Result<(), String> {
        while !bytes.is_empty() {
            let mut done = 0u32;
            unsafe { WriteFile(self.input, Some(bytes), Some(&mut done), None) }
                .map_err(|e| format!("write: {e}"))?;
            bytes = &bytes[done as usize..];
        }
        Ok(())
    }

    /// The root process's exit code once it has ended.
    pub fn exit_code(&self) -> Option<u32> {
        unsafe {
            if WaitForSingleObject(self.process, 0) != WAIT_OBJECT_0 {
                return None;
            }
            let mut code = 0u32;
            GetExitCodeProcess(self.process, &mut code).ok()?;
            Some(code)
        }
    }

    pub fn job_pids(&self) -> Vec<u32> {
        #[repr(C)]
        struct List {
            assigned: u32,
            in_list: u32,
            ids: [usize; 1024],
        }
        let mut list = List {
            assigned: 0,
            in_list: 0,
            ids: [0; 1024],
        };
        let ok = unsafe {
            QueryInformationJobObject(
                Some(self.job),
                JobObjectBasicProcessIdList,
                (&raw mut list).cast(),
                size_of::<List>() as u32,
                None,
            )
        };
        if ok.is_err() {
            return Vec::new();
        }
        let n = (list.in_list as usize).min(list.ids.len());
        list.ids[..n].iter().map(|&id| id as u32).collect()
    }

    pub fn active_count(&self) -> u32 {
        let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        let ok = unsafe {
            QueryInformationJobObject(
                Some(self.job),
                JobObjectBasicAccountingInformation,
                (&raw mut info).cast(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                None,
            )
        };
        if ok.is_ok() {
            info.ActiveProcesses
        } else {
            u32::MAX
        }
    }

    pub fn kill_tree(&self) -> Result<(), String> {
        unsafe { TerminateJobObject(self.job, 1) }.map_err(|e| format!("TerminateJobObject: {e}"))
    }

    /// Ends one process of the tree: `target` is a pid, or an image file name (the first such process). The pid
    /// must still be in this job when it is opened, since pids are reused.
    pub fn kill_one(&self, target: &str) -> Result<u32, String> {
        let pids = self.job_pids();
        let pid = match target.parse::<u32>() {
            Ok(pid) => pid,
            Err(_) => *pids
                .iter()
                .find(|&&pid| image_name(pid).is_some_and(|name| name.eq_ignore_ascii_case(target)))
                .ok_or_else(|| format!("no {target} in the tree"))?,
        };
        unsafe {
            let handle = OpenProcess(
                PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                pid,
            )
            .map_err(|e| format!("open {pid}: {e}"))?;
            let mut inside = windows::core::BOOL(0);
            let checked = IsProcessInJob(handle, Some(self.job), &mut inside);
            let result = if checked.is_ok() && inside.as_bool() {
                TerminateProcess(handle, 1).map_err(|e| format!("terminate {pid}: {e}"))
            } else {
                Err(format!("{pid} is not in this tree"))
            };
            let _ = CloseHandle(handle);
            result.map(|()| pid)
        }
    }

    /// Closes the pseudo console (its host ends and the reader sees the end of output) and every handle.
    pub fn close(mut self) {
        unsafe {
            ClosePseudoConsole(self.pcon);
            let _ = CloseHandle(self.input);
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        unsafe {
            let _ = CloseHandle(self.process);
            // The last handle to a kill-on-close job: anything still in it ends here.
            let _ = CloseHandle(self.job);
        }
    }
}

unsafe fn create_process(
    cmd: PWSTR,
    env: *const c_void,
    cwd: PCWSTR,
    startup: &STARTUPINFOW,
    pi: &mut PROCESS_INFORMATION,
) -> windows::core::Result<()> {
    unsafe {
        CreateProcessW(
            PCWSTR::null(),
            Some(cmd),
            None,
            None,
            false,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
            Some(env),
            cwd,
            startup,
            pi,
        )
    }
}

/// The image file name of a pid (`node.exe`), if the process can still be opened.
pub fn image_name(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        ok.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        Some(path.rsplit('\\').next().unwrap_or(&path).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_block_is_sorted_and_double_ended() {
        let vars = vec![
            (OsString::from("b"), OsString::from("2")),
            (OsString::from("A"), OsString::from("1")),
        ];
        let block = String::from_utf16(&env_block(&vars)).unwrap();
        assert_eq!(block, "A=1\0b=2\0\0");
        assert_eq!(env_block(&[]), vec![0, 0]);
    }
}
