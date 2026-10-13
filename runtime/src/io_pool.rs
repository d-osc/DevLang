// Async filesystem APIs use a bounded worker pool. Ordinary disk I/O stays on
// workers; scheduling and ready() do not block the interpreter on the operation.
use std::sync::atomic::{AtomicUsize as IoAtomicUsize, Ordering as IoOrdering};
use std::sync::{Condvar as IoCondvar, OnceLock as IoOnceLock};
const IO_QUEUE_BYTES: usize = 64 * 1024 * 1024;
static IO_PENDING_BYTES: IoAtomicUsize = IoAtomicUsize::new(0);
#[derive(Debug)]
struct IoFuture {
    result: Mutex<Option<TaskResult>>,
    signal: IoCondvar,
}
impl IoFuture {
    fn new() -> Self {
        Self {
            result: Mutex::new(None),
            signal: IoCondvar::new(),
        }
    }
    fn complete(&self, result: TaskResult) {
        if let Ok(mut slot) = self.result.lock() {
            *slot = Some(result);
            self.signal.notify_all();
        }
    }
    fn ready(&self) -> bool {
        self.result.try_lock().is_ok_and(|slot| slot.is_some())
    }
    fn join(&self) -> TaskResult {
        let mut slot = self.result.lock().map_err(|_| "I/O task lock poisoned")?;
        loop {
            if let Some(result) = slot.as_ref() {
                return result.clone();
            }
            slot = self
                .signal
                .wait(slot)
                .map_err(|_| "I/O task lock poisoned")?;
        }
    }
}
struct IoPermit(usize);
impl Drop for IoPermit {
    fn drop(&mut self) {
        IO_PENDING_BYTES.fetch_sub(self.0, IoOrdering::AcqRel);
    }
}
struct IoJob {
    name: String,
    args: Vec<Value>,
    future: Arc<IoFuture>,
    _permit: IoPermit,
}
static IO_POOL: IoOnceLock<Result<std::sync::mpsc::SyncSender<IoJob>, String>> = IoOnceLock::new();
fn io_pool() -> Result<&'static std::sync::mpsc::SyncSender<IoJob>, String> {
    IO_POOL
        .get_or_init(|| {
            let (sender, receiver) = std::sync::mpsc::sync_channel::<IoJob>(64);
            let receiver = Arc::new(Mutex::new(receiver));
            let workers = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
                .min(4);
            for i in 0..workers {
                let receiver = receiver.clone();
                std::thread::Builder::new()
                    .name(format!("dev-fs-{i}"))
                    .spawn(move || loop {
                        let job = {
                            let Ok(receiver) = receiver.lock() else {
                                break;
                            };
                            let Ok(job) = receiver.recv() else {
                                break;
                            };
                            job
                        };
                        let IoJob {
                            name,
                            args,
                            future,
                            _permit,
                        } = job;
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            crate::filesystem::call(&name, args)
                        }))
                        .unwrap_or_else(|_| Err("I/O worker panicked".into()));
                        future.complete(result);
                        drop(_permit);
                    })
                    .map_err(|e| format!("cannot start I/O worker: {e}"))?;
            }
            Ok(sender)
        })
        .as_ref()
        .map_err(Clone::clone)
}
impl Engine {
    fn fs_promise(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        let future = Arc::new(IoFuture::new());
        let method = match name {
            "readFile" => "readFileSync",
            "writeFile" => "writeFileSync",
            "appendFile" => "appendFileSync",
            "mkdir" => "mkdirSync",
            "readdir" => "readdirSync",
            "unlink" => "unlinkSync",
            "rename" => "renameSync",
            _ => return Err("unknown filesystem promise".into()),
        };
        let weight = args
            .iter()
            .map(|v| if let Value::Str(s) = v { s.len() } else { 64 })
            .fold(0usize, usize::saturating_add);
        if IO_PENDING_BYTES
            .fetch_update(IoOrdering::AcqRel, IoOrdering::Acquire, |n| {
                n.checked_add(weight).filter(|n| *n <= IO_QUEUE_BYTES)
            })
            .is_err()
        {
            future.complete(Err("filesystem promise byte budget exceeded; await pending tasks before scheduling more".into()));
        } else {
            let permit = IoPermit(weight);
            match io_pool() {
                Ok(pool) => {
                    let job = IoJob {
                        name: method.into(),
                        args,
                        future: future.clone(),
                        _permit: permit,
                    };
                    if pool.try_send(job).is_err() {
                        future.complete(Err("filesystem promise queue full; await pending tasks before scheduling more".into()));
                    }
                }
                Err(error) => {
                    drop(permit);
                    future.complete(Err(error));
                }
            }
        }
        Ok(Value::Task(
            Arc::new(TaskState {
                state: Mutex::new((None, None)),
                io: Some(future),
            }),
            ret,
        ))
    }
}
