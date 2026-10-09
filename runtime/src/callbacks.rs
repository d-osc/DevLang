type CallbackAction = Box<dyn Fn(Vec<Value>) -> Result<Value, String> + Send + Sync>;
struct CallbackData {
    params: Vec<Type>,
    result: Type,
    action: CallbackAction,
    error: std::sync::Mutex<HashMap<std::thread::ThreadId, String>>,
}
pub(crate) struct CallbackStorage {
    key: String,
    code: usize,
    _cif: Box<Cif>,
    data: Box<CallbackData>,
    allocation: *mut libffi::low::ffi_closure,
}
// Boxed CIF/data addresses stay stable. Registry ownership moves only under a
// mutex; dispatch reads the immutable CIF/action and uses a mutex for errors.
// Foreign callers must stop invoking pointers before the owning engine shuts down.
unsafe impl Send for CallbackStorage {}
impl Drop for CallbackStorage {
    fn drop(&mut self) {
        unsafe { libffi::low::closure_free(self.allocation) }
    }
}
unsafe extern "C" fn dispatch_callback(
    _: *mut libffi::low::ffi_cif,
    result: *mut c_void,
    args: *mut *mut c_void,
    userdata: *mut c_void,
) {
    let data = unsafe { &*(userdata as *const CallbackData) };
    // No Rust unwind may cross the C ABI. A callback error is reported by the caller.
    let call = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
        let pointers = if data.params.is_empty() {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(args, data.params.len()) }
        };
        let values = pointers
            .iter()
            .zip(&data.params)
            .map(|(p, t)| unsafe { crate::memory::read(*p as usize, t) })
            .collect::<Result<Vec<_>, _>>()?;
        let value = (data.action)(values)?;
        if data.result != Type::Void {
            match &value {
                Value::Int(
                    n,
                    Type::Int {
                        bits: 8 | 16 | 32, ..
                    },
                ) => unsafe { std::ptr::write(result as *mut isize, *n as isize) },
                Value::Bool(b) => unsafe { std::ptr::write(result as *mut usize, usize::from(*b)) },
                _ => unsafe { crate::memory::write(result as usize, &value, &mut vec![]) }?,
            }
        }
        Ok(())
    }));
    let error = match call {
        Ok(Ok(())) => None,
        Ok(Err(e)) => Some(e),
        Err(_) => Some("Dev callback panicked".into()),
    };
    if let Some(error) = error {
        if data.result != Type::Void {
            unsafe {
                std::ptr::write_bytes(
                    result as *mut u8,
                    0,
                    crate::memory::layout(&data.result)
                        .map(|(s, _)| s)
                        .unwrap_or(0),
                )
            };
        }
        if let Ok(mut e) = data.error.lock() {
            e.insert(std::thread::current().id(), error);
        }
    }
}
impl Native {
    pub fn callback(
        &mut self,
        key: String,
        t: &Type,
        action: CallbackAction,
    ) -> Result<Value, String> {
        let mut callbacks = self
            .callbacks
            .lock()
            .map_err(|_| "callback registry lock poisoned")?;
        if !key.is_empty() {
            if let Some(c) = callbacks.iter().find(|c| c.key == key) {
                return Ok(Value::Ptr(c.code, t.clone()));
            }
        }
        let Type::Callback(params, result) = t else {
            unreachable!()
        };
        let cif = Box::new(
            Cif::try_new(
                params.iter().map(ffi_type).collect::<Result<Vec<_>, _>>()?,
                ffi_type(result)?,
            )
            .map_err(|e| format!("invalid callback ABI: {e:?}"))?,
        );
        let data = Box::new(CallbackData {
            params: params.clone(),
            result: (**result).clone(),
            action,
            error: std::sync::Mutex::new(HashMap::new()),
        });
        let (allocation, code) =
            libffi::low::try_closure_alloc().ok_or("callback allocation failed")?;
        let status = unsafe {
            libffi::raw::ffi_prep_closure_loc(
                allocation,
                cif.as_raw_ptr(),
                Some(dispatch_callback),
                (&*data as *const CallbackData).cast_mut().cast(),
                code.as_mut_ptr(),
            )
        };
        if status != libffi::raw::ffi_status_FFI_OK {
            unsafe { libffi::low::closure_free(allocation) }
            return Err(format!("callback initialization failed: {status:?}"));
        }
        callbacks.push(CallbackStorage {
            key,
            code: code.as_ptr() as usize,
            _cif: cif,
            data,
            allocation,
        });
        Ok(Value::Ptr(code.as_ptr() as usize, t.clone()))
    }
    pub fn call_pointer(
        &mut self,
        address: usize,
        t: &Type,
        values: &[Value],
    ) -> Result<Value, String> {
        if address == 0 {
            return Err("null callback invocation".into());
        }
        let Type::Callback(params, result) = t else {
            unreachable!()
        };
        let name = format!("dev_callback_address_{address}_{t:?}");
        if !self.bindings.contains_key(&name) {
            let cif = Cif::try_new(
                params.iter().map(ffi_type).collect::<Result<Vec<_>, _>>()?,
                ffi_type(result)?,
            )
            .map_err(|e| format!("invalid callback ABI: {e:?}"))?;
            self.bindings.insert(
                name.clone(),
                (cif, CodePtr::from_ptr(address as *const c_void)),
            );
        }
        let f = Function {
            constraints: vec![],
            variadic: false,
            name,
            generics: vec![],
            main_default: false,
            params: params.iter().cloned().map(|t| (String::new(), t)).collect(),
            ret: (**result).clone(),
            body: None,
            exported: false,
            span: Default::default(),
        };
        self.call(&f, values)
    }
}
