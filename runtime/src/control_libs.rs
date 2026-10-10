struct CoreTimer {
    due: Instant,
    interval: Option<std::time::Duration>,
    callback: Value,
    referenced: bool,
    ticks: i64,
}
impl Engine {
    fn result_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        let name = ["attempt", "isOk", "isErr", "unwrapOr", "unwrap", "message"]
            .into_iter()
            .find(|base| name.starts_with(&format!("g_{base}_")))
            .unwrap_or(name);
        match name {
            "attempt" | "run" => {
                let ty = self.resolve_collection_element(&ret)?;
                let Type::Enum(_, variants) = &ty else {
                    return Err("result operation requires Result<T>".into());
                };
                if variants[0].1 == [Type::Void] {
                    return Err("use result.run for a void callback".into());
                }
                let outcome = self.invoke_core(args[0].clone(), vec![]);
                match outcome {
                    Ok(value) => Ok(Value::Enum(
                        0,
                        ty,
                        vec![if name == "run" {
                            Value::Bool(true)
                        } else {
                            value
                        }],
                    )),
                    Err(message) => Ok(Value::Enum(1, ty, vec![Value::Str(message)])),
                }
            }
            "raise" => Err(args[0].string()?.to_owned()),
            _ => {
                let Value::Enum(index, _, payload) = &args[0] else {
                    return Err("expected Result<T>".into());
                };
                match name {
                    "isOk" => Ok(Value::Bool(*index == 0)),
                    "isErr" => Ok(Value::Bool(*index == 1)),
                    "message" => Ok(Value::Str(if *index == 1 {
                        payload[0].string()?.into()
                    } else {
                        String::new()
                    })),
                    "unwrapOr" if *index == 1 => Ok(args[1].clone()),
                    "unwrap" | "unwrapOr" if *index == 0 => Ok(payload[0].clone()),
                    "unwrap" => Err(payload[0].string()?.into()),
                    _ => Err("unknown result API".into()),
                }
            }
        }
    }
    fn timers_active(&self) -> bool {
        self.core.timers.values().any(|timer| timer.referenced)
    }
    fn timers_tick(&mut self) -> Result<bool, String> {
        if self.core.timer_pumping {
            return Err("timer pump cannot be reentered".into());
        }
        self.core.timer_pumping = true;
        let result = (|| {
            let now = Instant::now();
            let mut due: Vec<_> = self
                .core
                .timers
                .iter()
                .filter(|(_, timer)| timer.due <= now)
                .map(|(id, timer)| (timer.due, *id))
                .collect();
            due.sort();
            let mut worked = false;
            for (_, id) in due {
                let Some(timer) = self.core.timers.get_mut(&id) else {
                    continue;
                };
                let once = timer.interval.is_none();
                timer.ticks = timer
                    .ticks
                    .checked_add(1)
                    .ok_or("timer tick count overflow")?;
                let callback = timer.callback.clone();
                let Value::Callable(_, _, _, Type::Function(params, _)) = &callback else {
                    return Err("invalid timer callback".into());
                };
                let value = Self::core_handle(id, params[0].clone());
                let outcome = self.invoke_core(callback, vec![value]);
                if once || outcome.is_err() {
                    self.core.timers.remove(&id);
                } else if let Some(timer) = self.core.timers.get_mut(&id) {
                    timer.due = Instant::now()
                        .checked_add(timer.interval.unwrap())
                        .ok_or("timer deadline overflow")?;
                }
                outcome?;
                worked = true;
            }
            Ok(worked)
        })();
        self.core.timer_pumping = false;
        result
    }
    fn timers_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        use std::time::Duration;
        match name {
            "setTimeout" | "setInterval" | "setImmediate" => {
                self.core_capacity()?;
                if self.core.timers.len() >= 64 {
                    return Err("at most 64 timers per interpreter".into());
                }
                let ms = if name == "setImmediate" {
                    0
                } else {
                    Self::core_size(&args[1], 300000)?
                };
                if name == "setInterval" && ms == 0 {
                    return Err("interval must be at least 1 ms".into());
                }
                let duration = Duration::from_millis(ms as u64);
                let id = self.http_id()?;
                self.core.timers.insert(
                    id,
                    CoreTimer {
                        due: Instant::now()
                            .checked_add(duration)
                            .ok_or("timer deadline overflow")?,
                        interval: (name == "setInterval").then_some(duration),
                        callback: args[0].clone(),
                        referenced: true,
                        ticks: 0,
                    },
                );
                Ok(Self::core_handle(id, ret))
            }
            "run" => {
                if self.core.timer_pumping {
                    return Err("timer pump cannot be reentered".into());
                }
                self.serve_http()?;
                Ok(Value::Void)
            }
            "sleep" => {
                let ms = Self::core_size(&args[0], 300000)?;
                std::thread::sleep(Duration::from_millis(ms as u64));
                Ok(Value::Void)
            }
            _ => {
                let id = Self::handle_id(&args[0])?;
                if ["clearTimeout", "clearInterval", "method_Timer_cancel"].contains(&name) {
                    return Ok(Value::Bool(self.core.timers.remove(&id).is_some()));
                }
                let timer = self
                    .core
                    .timers
                    .get_mut(&id)
                    .ok_or("timer is finished or belongs to another thread")?;
                match name {
                    "method_Timer_ref" => {
                        timer.referenced = true;
                        Ok(Value::Void)
                    }
                    "method_Timer_unref" => {
                        timer.referenced = false;
                        Ok(Value::Void)
                    }
                    "method_Timer_hasRef" => Ok(Value::Bool(timer.referenced)),
                    "method_Timer_ticks" => Ok(Value::Int(timer.ticks as i128, Type::i64())),
                    _ => Err("unknown timer API".into()),
                }
            }
        }
    }
}
