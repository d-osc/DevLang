use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as ChildOrdering};

struct ChildOutput {
    pid: u32,
    status: std::process::ExitStatus,
    timed_out: bool,
    killed: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
struct ChildGuard(command_group::GroupChild, bool);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !self.1 {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
struct CoreChild {
    pid: u32,
    cancel: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<Result<ChildOutput, String>>>,
}
impl Drop for CoreChild {
    fn drop(&mut self) {
        self.cancel.store(true, ChildOrdering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn child_reader<R: std::io::Read + Send + 'static>(
    mut input: R,
    used: Arc<AtomicUsize>,
    limit: usize,
) -> Result<std::thread::JoinHandle<Result<Vec<u8>, String>>, String> {
    std::thread::Builder::new()
        .name("dev-child-output".into())
        .spawn(move || {
            let mut bytes = Vec::new();
            let mut buffer = [0u8; 16384];
            loop {
                let n = input
                    .read(&mut buffer)
                    .map_err(|e| format!("child output read failed: {e}"))?;
                if n == 0 {
                    return Ok(bytes);
                }
                let before = used.fetch_add(n, ChildOrdering::Relaxed);
                if before.saturating_add(n) > limit {
                    return Err("child output exceeds maxBuffer".into());
                }
                bytes.extend_from_slice(&buffer[..n]);
            }
        })
        .map_err(|e| format!("cannot start child reader: {e}"))
}
fn child_join<T>(thread: std::thread::JoinHandle<Result<T, String>>) -> Result<T, String> {
    thread
        .join()
        .map_err(|_| "child I/O worker panicked".to_string())?
}
fn child_manage(
    mut group: ChildGuard,
    input: Vec<u8>,
    limit: usize,
    timeout: std::time::Duration,
    start: Instant,
    cancel: Arc<AtomicBool>,
) -> Result<ChildOutput, String> {
    use std::io::Write;
    let pid = group.0.id();
    // Take pipes before calling process-group methods; both output streams are
    // drained concurrently so either stream can exceed an OS pipe's capacity.
    let stdout = group
        .0
        .inner()
        .stdout
        .take()
        .ok_or("missing child stdout")?;
    let stderr = group
        .0
        .inner()
        .stderr
        .take()
        .ok_or("missing child stderr")?;
    let stdin = group.0.inner().stdin.take();
    let used = Arc::new(AtomicUsize::new(0));
    let out = child_reader(stdout, used.clone(), limit)?;
    let err = child_reader(stderr, used.clone(), limit)?;
    let writer = std::thread::Builder::new()
        .name("dev-child-input".into())
        .spawn(move || {
            if let Some(mut stdin) = stdin {
                if let Err(error) = stdin.write_all(&input) {
                    if error.kind() != std::io::ErrorKind::BrokenPipe {
                        return Err(format!("child stdin write failed: {error}"));
                    }
                }
            }
            Ok(())
        })
        .map_err(|e| format!("cannot start child writer: {e}"))?;
    let mut status = None;
    let mut timed_out = false;
    let mut failure = None;
    loop {
        if status.is_none() {
            match group.0.try_wait() {
                Ok(value) => status = value,
                Err(error) => {
                    failure = Some(format!("child wait failed: {error}"));
                    break;
                }
            }
        }
        if used.load(ChildOrdering::Relaxed) > limit {
            failure = Some("child output exceeds maxBuffer".into());
            break;
        }
        if status.is_some() && out.is_finished() && err.is_finished() && writer.is_finished() {
            break;
        }
        if cancel.load(ChildOrdering::Relaxed) {
            break;
        }
        if start.elapsed() >= timeout {
            timed_out = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    // Close the entire group/job before joining pipes. This also handles a child
    // that leaves a descendant holding inherited stdout/stderr open.
    if timed_out || cancel.load(ChildOrdering::Relaxed) || failure.is_some() {
        let _ = group.0.kill();
    }
    if status.is_none() {
        status = Some(
            group
                .0
                .wait()
                .map_err(|e| format!("child reap failed: {e}"))?,
        );
    }
    let stdout = child_join(out);
    let stderr = child_join(err);
    let input = child_join(writer);
    group.1 = true;
    if let Some(failure) = failure {
        return Err(failure);
    }
    input?;
    Ok(ChildOutput {
        pid,
        status: status.unwrap(),
        timed_out,
        killed: cancel.load(ChildOrdering::Relaxed),
        stdout: stdout?,
        stderr: stderr?,
    })
}
impl Engine {
    fn child_start(&mut self, args: &[Value]) -> Result<CoreChild, String> {
        use command_group::CommandGroup;
        use std::process::{Command, Stdio};
        let file = args[0].string()?;
        if file.is_empty() || file.len() > 32768 || file.contains('\0') {
            return Err("invalid child executable path".into());
        }
        // Windows can implicitly route batch files through cmd.exe; refuse that
        // path so argument arrays retain the direct-executable contract.
        if cfg!(windows)
            && ["bat", "cmd"].contains(
                &Path::new(file)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .as_str(),
            )
        {
            return Err("batch files require an explicit shell executable".into());
        }
        let arguments = cli_texts(&args[1])?;
        let options = &args[2];
        let cwd = crate::platform::field(options, "cwd")?.string()?;
        if cwd.len() > 32768 || cwd.contains('\0') {
            return Err("invalid child cwd".into());
        }
        let timeout = Self::core_size(crate::platform::field(options, "timeoutMs")?, 300000)?;
        if timeout == 0 {
            return Err("child timeoutMs must be between 1 and 300000".into());
        }
        let limit = Self::core_size(crate::platform::field(options, "maxBuffer")?, BASIC_LIMIT)?;
        if limit == 0 {
            return Err("child maxBuffer must be between 1 and 8 MiB".into());
        }
        let input = crate::filesystem::bytes(crate::platform::field(options, "input")?)?;
        if input.len() > BASIC_LIMIT {
            return Err("child input exceeds 8 MiB".into());
        }
        let mut command = Command::new(file);
        command
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if !cwd.is_empty() {
            command.current_dir(cwd);
        }
        let mut builder = command.group();
        #[cfg(windows)]
        builder.kill_on_drop(true).creation_flags(0x08000000); // CREATE_NO_WINDOW
        let group = ChildGuard(
            builder
                .spawn()
                .map_err(|e| format!("child spawn failed: {e}"))?,
            false,
        );
        let pid = group.0.id();
        let start = Instant::now();
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        let worker = std::thread::Builder::new()
            .name("dev-child-manager".into())
            .spawn(move || {
                child_manage(
                    group,
                    input,
                    limit,
                    std::time::Duration::from_millis(timeout as u64),
                    start,
                    stop,
                )
            })
            .map_err(|e| format!("cannot start child manager: {e}"))?;
        Ok(CoreChild {
            pid,
            cancel,
            worker: Some(worker),
        })
    }
    fn child_output(child: &mut CoreChild, ret: Type) -> Result<Value, String> {
        let output = child
            .worker
            .take()
            .ok_or("child was already waited")?
            .join()
            .map_err(|_| "child manager panicked".to_string())??;
        Ok(crate::platform::record(
            ret,
            vec![
                ("pid", Value::Int(output.pid as i128, Type::i64())),
                (
                    "code",
                    Value::Int(output.status.code().map_or(-1, i128::from), Type::i64()),
                ),
                (
                    "success",
                    Value::Bool(output.status.success() && !output.timed_out && !output.killed),
                ),
                ("timedOut", Value::Bool(output.timed_out)),
                ("killed", Value::Bool(output.killed)),
                ("stdout", crate::filesystem::bytes_value(output.stdout)),
                ("stderr", crate::filesystem::bytes_value(output.stderr)),
            ],
        ))
    }
    fn child_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        match name {
            "options" => {
                return Ok(crate::platform::record(
                    ret,
                    vec![
                        ("cwd", Value::Str(String::new())),
                        ("timeoutMs", Value::Int(30000, Type::i64())),
                        ("maxBuffer", Value::Int(1048576, Type::i64())),
                        ("input", crate::filesystem::bytes_value(Vec::new())),
                    ],
                ))
            }
            "spawn" | "execFileSync" => {
                self.core_capacity()?;
                if self.core.children.len() >= 32 {
                    return Err("at most 32 child handles per interpreter".into());
                }
                let mut child = self.child_start(&args)?;
                if name == "execFileSync" {
                    return Self::child_output(&mut child, ret);
                }
                let id = self.http_id()?;
                self.core.children.insert(id, child);
                return Ok(Self::core_handle(id, ret));
            }
            _ => {}
        }
        let id = Self::handle_id(&args[0])?;
        if name == "method_Child_wait" {
            let mut child = self
                .core
                .children
                .remove(&id)
                .ok_or("child was waited or belongs to another thread")?;
            return Self::child_output(&mut child, ret);
        }
        let child = self
            .core
            .children
            .get(&id)
            .ok_or("child was waited or belongs to another thread")?;
        match name {
            "method_Child_pid" => Ok(Value::Int(child.pid as i128, Type::i64())),
            "method_Child_ready" => Ok(Value::Bool(
                child
                    .worker
                    .as_ref()
                    .is_some_and(|worker| worker.is_finished()),
            )),
            "method_Child_kill" => {
                if child
                    .worker
                    .as_ref()
                    .is_some_and(|worker| worker.is_finished())
                {
                    return Ok(Value::Bool(false));
                }
                Ok(Value::Bool(
                    !child.cancel.swap(true, ChildOrdering::Relaxed),
                ))
            }
            _ => Err("unknown child_process API".into()),
        }
    }
}
