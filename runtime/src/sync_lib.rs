use std::collections::VecDeque;
use std::sync::{Condvar, MutexGuard};
use std::time::Duration;

pub(crate) enum SharedResource {
    Channel(SharedChannel),
    Lock(SharedLock),
    Cancel(SharedCancel),
}
impl std::fmt::Debug for SharedResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Channel(_) => "Channel",
            Self::Lock(_) => "Mutex",
            Self::Cancel(_) => "CancelToken",
        })
    }
}
pub(crate) struct SharedChannel {
    state: Mutex<QueueState>,
    changed: Condvar,
    capacity: usize,
}
struct QueueState {
    queue: VecDeque<(Value, usize)>,
    bytes: usize,
    closed: bool,
}
pub(crate) struct SharedLock {
    state: Mutex<LockState>,
    changed: Condvar,
}
struct LockState {
    value: Value,
    owner: Option<std::thread::ThreadId>,
    closed: bool,
}
pub(crate) struct SharedCancel {
    state: Mutex<bool>,
    changed: Condvar,
}

// The lease restores access even when a callback returns an error or unwinds.
// User code never executes while holding the Rust metadata mutex.
struct SharedLease(Arc<SharedResource>);
impl Drop for SharedLease {
    fn drop(&mut self) {
        if let SharedResource::Lock(lock) = self.0.as_ref() {
            let mut state = lock.state.lock().unwrap_or_else(|e| e.into_inner());
            state.owner = None;
            lock.changed.notify_all();
        }
    }
}
fn sync_deadline(value: &Value) -> Result<Instant, String> {
    let n = Engine::core_size(value, 300000)?;
    Instant::now()
        .checked_add(Duration::from_millis(n as u64))
        .ok_or_else(|| "sync timeout overflow".into())
}
fn sync_wait<'a, T>(
    changed: &Condvar,
    state: MutexGuard<'a, T>,
    deadline: Option<Instant>,
) -> Result<Option<MutexGuard<'a, T>>, String> {
    if let Some(deadline) = deadline {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Ok(None);
        };
        let (state, _) = changed
            .wait_timeout(state, remaining)
            .map_err(|_| "shared resource lock poisoned")?;
        // Always recheck the predicate after waking, including a timeout wake.
        Ok(Some(state))
    } else {
        Ok(Some(
            changed
                .wait(state)
                .map_err(|_| "shared resource lock poisoned")?,
        ))
    }
}
fn sync_lock_wait(
    lock: &SharedLock,
    deadline: Option<Instant>,
    allow_closed: bool,
) -> Result<MutexGuard<'_, LockState>, String> {
    let current = std::thread::current().id();
    let mut state = lock.state.lock().map_err(|_| "mutex lock poisoned")?;
    loop {
        if state.closed {
            if allow_closed {
                return Ok(state);
            }
            return Err("mutex is closed".into());
        }
        if state.owner == Some(current) {
            return Err("mutex callback cannot reenter the same mutex".into());
        }
        if state.owner.is_none() {
            return Ok(state);
        }
        state = sync_wait(&lock.changed, state, deadline)?.ok_or("mutex acquisition timed out")?;
    }
}

fn sync_payload_size(value: &Value) -> Result<usize, String> {
    fn visit(value: &Value, depth: usize, bytes: &mut usize) -> Result<(), String> {
        if depth > 128 {
            return Err("shared payload nesting exceeds 128".into());
        }
        *bytes = bytes
            .checked_add(64)
            .ok_or("shared payload size overflow")?;
        match value {
            Value::Int(..) | Value::Float(..) | Value::Bool(_) | Value::Void => {}
            Value::Str(text) => *bytes = bytes.checked_add(text.len()).ok_or("shared payload size overflow")?,
            Value::Array(items, _) | Value::Enum(_, _, items) => {
                for item in items { visit(item, depth + 1, bytes)?; }
            }
            Value::Vector(items, _) => for item in items.iter() { visit(item, depth + 1, bytes)?; },
            Value::Slice(items, offset, len, _) => for item in &items[*offset..*offset + *len] { visit(item, depth + 1, bytes)?; },
            Value::Record(fields, _) => for (_, item) in fields { visit(item, depth + 1, bytes)?; },
            Value::Ref(item, _) => visit(item, depth + 1, bytes)?,
            Value::Map(items, _) => for (key, item) in items.entries() { visit(key, depth + 1, bytes)?; visit(item, depth + 1, bytes)?; },
            Value::Json(json, _) => {
                fn json_size(value: &serde_json::Value, depth: usize, bytes: &mut usize) -> Result<(), String> {
                    if depth > 128 { return Err("shared payload nesting exceeds 128".into()); }
                    *bytes = bytes.checked_add(64).ok_or("shared payload size overflow")?;
                    match value {
                        serde_json::Value::String(s) => *bytes = bytes.checked_add(s.len()).ok_or("shared payload size overflow")?,
                        serde_json::Value::Array(xs) => for x in xs { json_size(x, depth + 1, bytes)?; },
                        serde_json::Value::Object(xs) => for (k,v) in xs { *bytes = bytes.checked_add(k.len()).ok_or("shared payload size overflow")?; json_size(v, depth + 1, bytes)?; },
                        serde_json::Value::Number(n) => *bytes = bytes.checked_add(n.to_string().len()).ok_or("shared payload size overflow")?,
                        _ => {}
                    }
                    if *bytes > BASIC_LIMIT { return Err("shared payload exceeds 8 MiB model budget".into()); }
                    Ok(())
                }
                json_size(json, depth, bytes)?;
            }
            Value::Ptr(..) | Value::Callable(..) | Value::Task(..) | Value::Shared(..) => return Err("shared payload must be data; pointers, functions, tasks and sync handles are not supported".into()),
        }
        if *bytes > BASIC_LIMIT {
            return Err("shared payload exceeds 8 MiB model budget".into());
        }
        Ok(())
    }
    let mut bytes = 0;
    visit(value, 0, &mut bytes)?;
    Ok(bytes)
}
impl Engine {
    fn sync_resource(&mut self, resource: SharedResource, ret: Type) -> Result<Value, String> {
        self.core.sync_resources.retain(|r| r.strong_count() > 0);
        self.core_capacity()?;
        if self.core.sync_resources.len() >= 64 {
            return Err("at most 64 live sync resources per creating interpreter".into());
        }
        let resource = Arc::new(resource);
        self.core.sync_resources.push(Arc::downgrade(&resource));
        Ok(Value::Shared(
            resource,
            self.resolve_collection_element(&ret)?,
        ))
    }
    fn sync_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        let bases = [
            "channel",
            "mutex",
            "method_Channel_send",
            "method_Channel_sendTimeout",
            "method_Channel_trySend",
            "method_Channel_receive",
            "method_Channel_receiveTimeout",
            "method_Channel_tryReceive",
            "method_Channel_close",
            "method_Channel_isClosed",
            "method_Channel_len",
            "method_Channel_capacity",
            "method_Mutex_get",
            "method_Mutex_set",
            "method_Mutex_update",
            "method_Mutex_updateTimeout",
            "method_Mutex_close",
        ];
        let name = bases
            .into_iter()
            .find(|base| name.starts_with(&format!("g_{base}_")))
            .unwrap_or(name);
        match name {
            "channel" => {
                let capacity = Self::core_size(&args[0], 65536)?;
                if capacity == 0 {
                    return Err("channel capacity must be 1..65536".into());
                }
                return self.sync_resource(
                    SharedResource::Channel(SharedChannel {
                        state: Mutex::new(QueueState {
                            queue: VecDeque::new(),
                            bytes: 0,
                            closed: false,
                        }),
                        changed: Condvar::new(),
                        capacity,
                    }),
                    ret,
                );
            }
            "mutex" => {
                sync_payload_size(&args[0])?;
                return self.sync_resource(
                    SharedResource::Lock(SharedLock {
                        state: Mutex::new(LockState {
                            value: args[0].clone(),
                            owner: None,
                            closed: false,
                        }),
                        changed: Condvar::new(),
                    }),
                    ret,
                );
            }
            "token" => {
                return self.sync_resource(
                    SharedResource::Cancel(SharedCancel {
                        state: Mutex::new(false),
                        changed: Condvar::new(),
                    }),
                    ret,
                )
            }
            _ => {}
        }
        let Value::Shared(resource, _) = &args[0] else {
            return Err(
                "sync handle must be created by sync.channel, sync.mutex or sync.token".into(),
            );
        };
        let resource = resource.clone();
        match resource.as_ref() {
            SharedResource::Cancel(token) => {
                let mut state = token
                    .state
                    .lock()
                    .map_err(|_| "cancel token lock poisoned")?;
                match name {
                    "method_CancelToken_cancel" => {
                        let changed = !*state;
                        *state = true;
                        token.changed.notify_all();
                        Ok(Value::Bool(changed))
                    }
                    "method_CancelToken_isCancelled" => Ok(Value::Bool(*state)),
                    "method_CancelToken_check" => {
                        if *state {
                            Err("operation cancelled".into())
                        } else {
                            Ok(Value::Void)
                        }
                    }
                    "method_CancelToken_wait" => {
                        let deadline = sync_deadline(&args[1])?;
                        while !*state {
                            match sync_wait(&token.changed, state, Some(deadline))? {
                                Some(s) => state = s,
                                None => return Ok(Value::Bool(false)),
                            }
                        }
                        Ok(Value::Bool(true))
                    }
                    _ => Err("unknown cancellation API".into()),
                }
            }
            SharedResource::Channel(channel) => {
                let sending = [
                    "method_Channel_send",
                    "method_Channel_sendTimeout",
                    "method_Channel_trySend",
                ]
                .contains(&name);
                let receiving = [
                    "method_Channel_receive",
                    "method_Channel_receiveTimeout",
                    "method_Channel_tryReceive",
                ]
                .contains(&name);
                let deadline = match name {
                    "method_Channel_sendTimeout" => Some(sync_deadline(&args[2])?),
                    "method_Channel_receiveTimeout" => Some(sync_deadline(&args[1])?),
                    "method_Channel_trySend" | "method_Channel_tryReceive" => Some(Instant::now()),
                    _ => None,
                };
                let bytes = if sending {
                    sync_payload_size(&args[1])?
                } else {
                    0
                };
                let output_type = if receiving {
                    self.resolve_collection_element(&ret)?
                } else {
                    ret
                };
                let mut state = channel.state.lock().map_err(|_| "channel lock poisoned")?;
                loop {
                    if sending {
                        if state.closed {
                            return Ok(Value::Bool(false));
                        }
                        if state.queue.len() < channel.capacity
                            && state.bytes + bytes <= BASIC_LIMIT
                        {
                            state
                                .queue
                                .try_reserve(1)
                                .map_err(|_| "channel allocation failed")?;
                            state.queue.push_back((args[1].clone(), bytes));
                            state.bytes += bytes;
                            channel.changed.notify_all();
                            return Ok(Value::Bool(true));
                        }
                    } else if receiving {
                        if let Some((item, bytes)) = state.queue.pop_front() {
                            state.bytes -= bytes;
                            channel.changed.notify_all();
                            return Ok(Value::Enum(0, output_type, vec![item]));
                        }
                        if state.closed {
                            return Ok(Value::Enum(2, output_type, vec![]));
                        }
                    } else {
                        return match name {
                            "method_Channel_close" => {
                                let changed = !state.closed;
                                state.closed = true;
                                channel.changed.notify_all();
                                Ok(Value::Bool(changed))
                            }
                            "method_Channel_isClosed" => Ok(Value::Bool(state.closed)),
                            "method_Channel_len" => {
                                Ok(Value::Int(state.queue.len() as i128, Type::i64()))
                            }
                            "method_Channel_capacity" => {
                                Ok(Value::Int(channel.capacity as i128, Type::i64()))
                            }
                            _ => Err("unknown channel API".into()),
                        };
                    }
                    state = match sync_wait(&channel.changed, state, deadline)? {
                        Some(state) => state,
                        None => {
                            return if sending {
                                Ok(Value::Bool(false))
                            } else {
                                Ok(Value::Enum(1, output_type, vec![]))
                            }
                        }
                    };
                }
            }
            SharedResource::Lock(lock) => {
                let deadline = if name == "method_Mutex_updateTimeout" {
                    Some(sync_deadline(&args[2])?)
                } else {
                    None
                };
                let mut state = sync_lock_wait(lock, deadline, name == "method_Mutex_close")?;
                match name {
                    "method_Mutex_get" => Ok(state.value.clone()),
                    "method_Mutex_set" => {
                        sync_payload_size(&args[1])?;
                        state.value = args[1].clone();
                        Ok(Value::Void)
                    }
                    "method_Mutex_close" => {
                        let changed = !state.closed;
                        state.closed = true;
                        state.value = Value::Void;
                        lock.changed.notify_all();
                        Ok(Value::Bool(changed))
                    }
                    "method_Mutex_update" | "method_Mutex_updateTimeout" => {
                        let previous = state.value.clone();
                        state.owner = Some(std::thread::current().id());
                        drop(state);
                        let lease = SharedLease(resource.clone());
                        let updated = self.invoke_core(args[1].clone(), vec![previous.clone()])?;
                        let updated = convert(updated, &previous.ty(), false)?;
                        sync_payload_size(&updated)?;
                        lock.state.lock().map_err(|_| "mutex lock poisoned")?.value =
                            updated.clone();
                        drop(lease);
                        Ok(updated)
                    }
                    _ => Err("unknown mutex API".into()),
                }
            }
        }
    }
}
