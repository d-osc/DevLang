use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub(crate) struct ProtoProperty {
    pub value: Option<Value>,
    pub get: Option<Value>,
    pub set: Option<Value>,
    pub writable: bool,
    pub enumerable: bool,
    pub configurable: bool,
}
#[derive(Debug)]
pub(crate) struct ProtoObject {
    pub properties: BTreeMap<String, ProtoProperty>,
    pub parent: Option<Value>,
    pub extensible: bool,
}
fn object_type() -> Type {
    Type::Record("dev_dynamic_value".into(), vec![])
}
fn object_data(value: Value) -> ProtoProperty {
    ProtoProperty {
        value: Some(value),
        get: None,
        set: None,
        writable: true,
        enumerable: true,
        configurable: true,
    }
}
fn new_object(parent:Option<Value>)->Value {let state=Arc::new(Mutex::new(ProtoObject{properties:BTreeMap::new(),parent,extensible:true}));prototype_register(&state);Value::PrototypeObject(state,object_type())}

fn object_function(value: Value) -> Value {
    if matches!(value, Value::PrototypeFunction(..)) {
        value
    } else {
        Value::PrototypeFunction(Box::new(value), object_type())
    }
}
fn object_sequence(value: &Value) -> Result<Vec<Value>, String> {
    match value {
        Value::PrototypeArray(xs, _) => Ok(xs.as_ref().clone()),
        Value::Array(xs, _) => Ok(xs.clone()),
        Value::Vector(xs, _) => Ok(xs.as_ref().clone()),
        Value::Slice(xs, o, n, _) => Ok(xs[*o..*o + *n].to_vec()),
        Value::Json(node, _) => node
            .as_array()
            .ok_or("expected iterable collection")?
            .iter()
            .map(|v| Ok(crate::json::unwrap(v.clone(), object_type())))
            .collect(),
        _ => Err("expected Array, Vec, Slice or DON array".into()),
    }
}
impl Engine {
    fn object_argument(&mut self, frame: &mut Frame, e: &Expr) -> Result<Value, String> {
        match &e.kind {
            ExprKind::Object(_, fields) => {
                let parent = self.object_root();
                let object = new_object(Some(parent));
                let state = self.object_state(&object)?;
                for (key, e) in fields {
                    let value = self.object_argument(frame, e)?;
                    state
                        .lock()
                        .map_err(|_| "object lock poisoned")?
                        .properties
                        .insert(key.clone(), object_data(value));
                }
                Ok(object)
            }
            ExprKind::Array(items) => {
                let values = items
                    .iter()
                    .map(|e| self.object_argument(frame, e))
                    .collect::<Result<Vec<_>, _>>()?;
                let n = values.len();
                Ok(Value::Array(
                    values,
                    Type::Array(Box::new(object_type()), n),
                ))
            }
            _ => self.eval(frame, e, None),
        }
    }
    fn object_root(&mut self) -> Value {
        if let Some(value) = &self.object_prototype {
            return value.clone();
        }
        let root = new_object(None);
        self.object_prototype = Some(root.clone());
        let state = self.object_state(&root).unwrap();
        let _world=prototype_read();
        for name in ["constructor", "toString", "valueOf"] {
            let ret = if name == "toString" {
                Type::Str
            } else {
                object_type()
            };
            let function = object_function(Value::Callable(
                "$object_builtin".into(),
                name.into(),
                None,
                Type::Function(vec![], Box::new(ret)),
            ));
            let mut property = object_data(function);
            property.enumerable = false;
            state
                .lock()
                .unwrap()
                .properties
                .insert(name.into(), property);
        }
        root
    }
    fn object_state(&mut self, value: &Value) -> Result<Arc<Mutex<ProtoObject>>, String> {
        match value {
            Value::PrototypeObject(state,_)=>Ok(state.clone()),
            Value::PrototypeFunction(function,_)=>self.object_state(function),
            Value::Callable(module,name,context,_)=>{
                let key=format!("{module}:{name}:{:p}",context.as_ref().map(Arc::as_ptr).unwrap_or(std::ptr::null()));
                if let Some(state)=self.function_objects.get(&key){return Ok(state.clone());}
                let root=self.object_root();let prototype=if module=="$object_builtin"&&name=="constructor"{root.clone()}else{new_object(Some(root.clone()))};let prototype_state=self.object_state(&prototype)?;
                let state=Arc::new(Mutex::new(ProtoObject{properties:BTreeMap::new(),parent:Some(root),extensible:true}));prototype_register(&state);
                let _world=prototype_read();let mut ctor=object_data(object_function(value.clone()));ctor.enumerable=false;prototype_state.lock().map_err(|_|"object lock poisoned")?.properties.insert("constructor".into(),ctor);
                let mut property=object_data(prototype);property.enumerable=false;property.configurable=false;if module=="$object_builtin"&&name=="constructor"{property.writable=false;}state.lock().unwrap().properties.insert("prototype".into(),property);
                if module=="$object_builtin"&&name=="constructor"{for name in ["assign","create","defineProperties","defineProperty","delete","entries","freeze","fromEntries","getOwnPropertyDescriptor","getOwnPropertyDescriptors","getOwnPropertyNames","groupBy","isExtensible","isFrozen","isSealed","keys","preventExtensions","seal","toString","valueOf","values","construct"]{let value=object_function(Value::Callable("$object_static".into(),name.into(),None,Type::Function(vec![],Box::new(object_type()))));let mut property=object_data(value);property.enumerable=false;state.lock().unwrap().properties.insert(name.into(),property);}}
                if let Some(context)=context{self.function_contexts.insert(key.clone(),Arc::downgrade(context));}self.function_objects.insert(key,state.clone());Ok(state)
            }
            _=>Err("operation requires a prototype Object; construct with Object({...}) or Object.create()".into()),
        }
    }
    fn object_own(&mut self, value: &Value) -> Result<BTreeMap<String, ProtoProperty>, String> {
        if matches!(
            value,
            Value::PrototypeObject(..) | Value::PrototypeFunction(..) | Value::Callable(..)
        ) {
            return Ok(self
                .object_state(value)?
                .lock()
                .map_err(|_| "object lock poisoned")?
                .properties
                .clone());
        }
        match value {
            Value::Record(fields, _) => Ok(fields
                .iter()
                .map(|(k, v)| (k.clone(), object_data(v.clone())))
                .collect()),
            Value::Json(node, _) => Ok(node
                .as_object()
                .ok_or("expected object")?
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        object_data(crate::json::unwrap(v.clone(), object_type())),
                    )
                })
                .collect()),
            _ => Err("expected object".into()),
        }
    }
    fn object_lookup(
        &mut self,
        object: &Value,
        key: &str,
    ) -> Result<Option<ProtoProperty>, String> {
        let mut cursor = Some(object.clone());
        for _ in 0..128 {
            let Some(value) = cursor else {
                return Ok(None);
            };
            if let Some(property) = self.object_own(&value)?.get(key) {
                return Ok(Some(property.clone()));
            }
            cursor = if matches!(
                &value,
                Value::PrototypeObject(..) | Value::PrototypeFunction(..) | Value::Callable(..)
            ) {
                self.object_state(&value)?
                    .lock()
                    .map_err(|_| "object lock poisoned")?
                    .parent
                    .clone()
            } else {
                None
            };
        }
        Err("prototype chain exceeds 128 levels".into())
    }
    fn object_get(&mut self, object: Value, key: &str, span: Span) -> Result<Value, String> {
        let property = self
            .object_lookup(&object, key)?
            .ok_or_else(|| format!("missing object property '{key}'"))?;
        let value=if let Some(get)=property.get {self.object_invoke(get,vec![object],span)?}else{
            property.value.unwrap_or_else(||Value::Json(Arc::new(serde_json::Value::Null),object_type()))
        };
        Ok(if matches!(value,Value::Callable(..)){object_function(value)}else{value})
    }
    fn object_invoke(
        &mut self,
        callable: Value,
        values: Vec<Value>,
        span: Span,
    ) -> Result<Value, String> {
        let callable = match callable {
            Value::PrototypeFunction(value, _) => *value,
            value => value,
        };
        let Value::Callable(module, name, context, Type::Function(params, _)) = callable else {
            return Err("object property is not callable".into());
        };
        if module == "$object_builtin" {
            return self.object_builtin(&[name], values, span);
        }
        if module == "$object_static" {
            return self.object_api(&name, values, span);
        }
        if params.len() != values.len() {
            return Err(
                "wrong object method argument count; methods receive this as their first parameter"
                    .into(),
            );
        }
        let mut args = Vec::new();
        if let Some(value) = context {
            let inner = match value.ty() {
                Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                t => t,
            };
            args.push(Value::Ref(value, Type::Ref(Box::new(inner))));
        }
        for (value, ty) in values.into_iter().zip(params) {
            args.push(if matches!(&ty,Type::Record(n,_)|Type::Nominal(n) if n=="dev_dynamic_value" || n=="dev_dynamic_don_value") {self.object_box(value)?} else {convert(value,&ty,false)?});
        }
        self.call(&module, &[name], args, span)
    }
    fn object_box(&mut self, value: Value) -> Result<Value, String> {
        if matches!(
            value,
            Value::PrototypeObject(..)
                | Value::PrototypeFunction(..)
                | Value::PrototypeArray(..)
                | Value::Json(..)
        ) {
            return Ok(value);
        }
        if matches!(value,Value::Array(..)|Value::Vector(..)|Value::Slice(..)){
            let mut values=Vec::new();for item in object_sequence(&value)?{values.push(self.object_box(item)?);}return Ok(Value::PrototypeArray(Arc::new(values),object_type()));
        }
        if matches!(value, Value::Callable(..)) {
            return Ok(object_function(value));
        }
        Ok(Value::Json(
            Arc::new(crate::json::encode(&value, 0)?),
            object_type(),
        ))
    }
    fn object_method(
        &mut self,
        object: Value,
        name: &str,
        values: Vec<Value>,
        span: Span,
    ) -> Result<Value, String> {
        let callable = self.object_get(object.clone(), name, span)?;
        let original = match &callable {
            Value::PrototypeFunction(v, _) => v.as_ref(),
            v => v,
        };
        if matches!(original,Value::Callable(module,name,_,_) if module=="$object_static" || (module=="$object_builtin"&&name=="constructor"))
        {
            return self.object_invoke(callable, values, span);
        }
        let mut args = vec![object];
        args.extend(values);
        self.object_invoke(callable, args, span)
    }
    fn object_builtin(
        &mut self,
        name: &[String],
        values: Vec<Value>,
        _span: Span,
    ) -> Result<Value, String> {
        match name.first().map(String::as_str) {
            Some("constructor") => self.object_api("object", values, _span),
            Some("toString") => Ok(Value::Str(
                if values.first().is_some_and(|v| {
                    matches!(v, Value::PrototypeFunction(..) | Value::Callable(..))
                }) {
                    "[object Function]"
                } else {
                    "[object Object]"
                }
                .into(),
            )),
            Some("valueOf") => values
                .into_iter()
                .next()
                .ok_or("valueOf needs a receiver".into()),
            _ => Err("unknown Object builtin".into()),
        }
    }
    fn object_set(
        &mut self,
        object: &Value,
        key: &str,
        value: Value,
        span: Span,
    ) -> Result<(), String> {
        let state = self.object_state(object)?;
        if let Some(property) = self.object_lookup(object, key)? {
            if let Some(set) = property.set {
                self.object_invoke(set, vec![object.clone(), value], span)?;
                return Ok(());
            }
            if property.get.is_some() || !property.writable {
                return Err(format!("object property '{key}' is read-only"));
            }
        }
        let _world=prototype_read();
        let mut state = state.lock().map_err(|_| "object lock poisoned")?;
        if let Some(property) = state.properties.get_mut(key) {
            let retired = property.value.replace(value);
            drop(state);
            drop(_world);
            drop(retired);
            return Ok(());
        }
        if !state.extensible {
            return Err("object is not extensible".into());
        }
        state.properties.insert(key.into(), object_data(value));
        Ok(())
    }
    fn object_assign(
        &mut self,
        frame: &mut Frame,
        target: &Expr,
        op: &str,
        rhs: &Expr,
    ) -> Result<bool, String> {
        fn has_object(frame: &Frame, e: &Expr) -> bool {
            match &e.kind {
                ExprKind::Name(names) if names.len() == 1 => frame
                    .scopes
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(&names[0]))
                    .is_some_and(|v| {
                        matches!(
                            v,
                            Value::PrototypeObject(..)
                                | Value::PrototypeFunction(..)
                                | Value::Callable(..)
                        )
                    }),
                ExprKind::Field(base, _) | ExprKind::Index(base, _) => has_object(frame, base),
                ExprKind::Call(names, _) => names.first().is_some_and(|n| n == "$object"),
                ExprKind::Callable(..) => true,
                _ => false,
            }
        }
        if !has_object(frame, target) {
            return Ok(false);
        }
        let (base, key) = match &target.kind {
            ExprKind::Field(base, key) => (base.as_ref(), key.clone()),
            ExprKind::Index(base, index) => {
                let value = self.eval(frame, base, None)?;
                if !matches!(
                    &value,
                    Value::PrototypeObject(..) | Value::PrototypeFunction(..) | Value::Callable(..)
                ) {
                    return Ok(false);
                }
                let key = self
                    .eval(frame, index, Some(&Type::Str))?
                    .string()?
                    .to_string();
                let rhs = self.eval(frame, rhs, None)?;
                let result = if op == "=" {
                    rhs
                } else {
                    let old = self.object_get(value.clone(), &key, target.span)?;
                    self.binary(op.trim_end_matches('='), old, rhs)?
                };
                self.object_set(&value, &key, result, target.span)?;
                return Ok(true);
            }
            _ => return Ok(false),
        };
        let object = self.eval(frame, base, None)?;
        if !matches!(
            &object,
            Value::PrototypeObject(..) | Value::PrototypeFunction(..) | Value::Callable(..)
        ) {
            return Ok(false);
        }
        let value = self.eval(frame, rhs, None)?;
        let value = if op == "=" {
            value
        } else {
            let old = self.object_get(object.clone(), &key, target.span)?;
            self.binary(op.trim_end_matches('='), old, value)?
        };
        self.object_set(&object, &key, value, target.span)?;
        Ok(true)
    }
    fn object_descriptor(&mut self, value: &Value) -> Result<BTreeMap<String, Value>, String> {
        let mut fields = BTreeMap::new();
        for key in [
            "value",
            "get",
            "set",
            "writable",
            "enumerable",
            "configurable",
        ] {
            if self.object_lookup(value, &key)?.is_some() {
                fields.insert(
                    key.into(),
                    self.object_get(value.clone(), key, Span::default())?,
                );
            }
        }
        Ok(fields)
    }
    fn object_define(
        &mut self,
        object: &Value,
        key: &str,
        descriptor: &Value,
    ) -> Result<(), String> {
        let fields = if matches!(
            descriptor,
            Value::Callable(..) | Value::PrototypeFunction(..)
        ) {
            BTreeMap::from([
                ("value".into(), descriptor.clone()),
                ("writable".into(), Value::Bool(true)),
                ("configurable".into(), Value::Bool(true)),
            ])
        } else {
            self.object_descriptor(descriptor)?
        };
        if (fields.contains_key("get") || fields.contains_key("set"))
            && (fields.contains_key("value") || fields.contains_key("writable"))
        {
            return Err("descriptor cannot mix value/writable with get/set".into());
        }
        for name in ["get", "set"] {
            if let Some(value) = fields.get(name) {
                if !(matches!(value, Value::Callable(..) | Value::PrototypeFunction(..))
                    || matches!(value,Value::Json(node,_) if node.is_null()))
                {
                    return Err("descriptor getter/setter must be functions or null".into());
                }
            }
        }
        let state = self.object_state(object)?;
        let _world=prototype_read();
        let mut state = state.lock().map_err(|_| "object lock poisoned")?;
        let old = state.properties.get(key).cloned();
        if old.is_none() && !state.extensible {
            return Err("object is not extensible".into());
        }
        let accessor = fields.contains_key("get") || fields.contains_key("set");
        let data = fields.contains_key("value") || fields.contains_key("writable");
        let mut property = old.clone().unwrap_or(ProtoProperty {
            value: Some(Value::Json(
                Arc::new(serde_json::Value::Null),
                object_type(),
            )),
            get: None,
            set: None,
            writable: false,
            enumerable: false,
            configurable: false,
        });
        if accessor {
            property.value = None;
            property.writable = false;
        }
        if data {
            property.get = None;
            property.set = None;
        }
        for (name, slot) in [
            ("writable", &mut property.writable),
            ("enumerable", &mut property.enumerable),
            ("configurable", &mut property.configurable),
        ] {
            if let Some(value) = fields.get(name) {
                *slot = value.boolean()?;
            }
        }
        if let Some(value) = fields.get("value") {
            property.value = Some(value.clone());
        }
        for (name, slot) in [("get", &mut property.get), ("set", &mut property.set)] {
            if let Some(value) = fields.get(name) {
                *slot = if matches!(value,Value::Json(node,_) if node.is_null()) {
                    None
                } else {
                    Some(value.clone())
                };
            }
        }
        if let Some(old) = old {
            if !old.configurable {
                if property.configurable
                    || old.enumerable != property.enumerable
                    || old.value.is_some() != property.value.is_some()
                    || (old.get.is_some() != property.get.is_some())
                    || (old.set.is_some() != property.set.is_some())
                {
                    return Err("cannot redefine non-configurable property".into());
                }
                if old.value.is_some()
                    && !old.writable
                    && (property.writable
                        || (fields.contains_key("value")
                            && !object_same(
                                old.value.as_ref().unwrap(),
                                property.value.as_ref().unwrap(),
                            )))
                {
                    return Err("cannot redefine non-writable property".into());
                }
                if old.value.is_none()
                    && ((fields.contains_key("get")
                        && !match (&old.get, &property.get) {
                            (None, None) => true,
                            (Some(a), Some(b)) => object_same(a, b),
                            _ => false,
                        })
                        || (fields.contains_key("set")
                            && !match (&old.set, &property.set) {
                                (None, None) => true,
                                (Some(a), Some(b)) => object_same(a, b),
                                _ => false,
                            }))
                {
                    return Err("cannot redefine non-configurable accessor".into());
                }
            }
        }
        let retired = state.properties.insert(key.into(), property);
        drop(state);
        drop(_world);
        drop(retired);
        Ok(())
    }
    fn descriptor_value(&mut self, property: ProtoProperty) -> Result<Value, String> {
        let object = new_object(None);
        let state = self.object_state(&object)?;
        let _world=prototype_read();
        let mut fields = state.lock().map_err(|_| "object lock poisoned")?;
        for (key, value) in [
            ("enumerable", Value::Bool(property.enumerable)),
            ("configurable", Value::Bool(property.configurable)),
        ] {
            fields.properties.insert(key.into(), object_data(value));
        }
        if let Some(value) = property.value {
            fields.properties.insert("value".into(), object_data(value));
            fields.properties.insert(
                "writable".into(),
                object_data(Value::Bool(property.writable)),
            );
        } else {
            for (key, value) in [("get", property.get), ("set", property.set)] {
                fields.properties.insert(
                    key.into(),
                    object_data(value.unwrap_or_else(|| {
                        Value::Json(Arc::new(serde_json::Value::Null), object_type())
                    })),
                );
            }
        }
        drop(fields);
        Ok(object)
    }
    fn object_api(&mut self, name: &str, args: Vec<Value>, span: Span) -> Result<Value, String> {
        let arity = match name {
            "prototype" | "constructor" => Some(0),
            "static" => Some(1),
            "defineProperty" => Some(3),
            "defineProperties" | "delete" | "getOwnPropertyDescriptor" | "groupBy" => Some(2),
            "keys"
            | "values"
            | "entries"
            | "freeze"
            | "seal"
            | "preventExtensions"
            | "isExtensible"
            | "isFrozen"
            | "isSealed"
            | "getOwnPropertyNames"
            | "getOwnPropertyDescriptors"
            | "fromEntries"
            | "toString"
            | "valueOf" => Some(1),
            "array" | "object" | "create" | "assign" | "construct" => None,
            _ => return Err(format!("unknown Object API '{name}'")),
        };
        if arity.is_some_and(|n| args.len() != n) {
            return Err(format!("wrong Object.{name} argument count"));
        }
        match name {
            "array" => {
                let values = if args.len() == 1 && args[0].ty().numeric() {
                    let length = match &args[0] {
                        Value::Int(n, _) => *n,
                        Value::Float(n, _) if n.is_finite() && n.fract() == 0.0 => *n as i128,
                        _ => return Err("Array length must be a non-negative integer".into()),
                    };
                    if !(0..=1_000_000).contains(&length) {
                        return Err("Array length must be between 0 and 1000000".into());
                    }
                    vec![
                        Value::Json(Arc::new(serde_json::Value::Null), object_type());
                        length as usize
                    ]
                } else {
                    args.into_iter()
                        .map(|v| self.object_box(v))
                        .collect::<Result<Vec<_>, _>>()?
                };
                Ok(Value::Vector(
                    Arc::new(values),
                    Type::Vector(Box::new(object_type())),
                ))
            }
            "static" => {
                let ctor = self.object_api("constructor", vec![], span)?;
                self.object_get(ctor, args[0].string()?, span)
            }
            "prototype" => Ok(self.object_root()),
            "constructor" => {
                let root = self.object_root();
                self.object_get(root, "constructor", span)
            }
            "object" => {
                if args.len() > 1 {
                    return Err("Object accepts zero or one data object".into());
                }
                if let Some(value) = args.first() {
                    if matches!(
                        value,
                        Value::PrototypeObject(..) | Value::PrototypeFunction(..)
                    ) {
                        return Ok(value.clone());
                    }
                    if matches!(value, Value::Callable(..)) {
                        return Ok(object_function(value.clone()));
                    }
                }
                let root = self.object_root();
                let object = new_object(Some(root));
                if let Some(source) = args.first() {
                    for (key, property) in self.object_own(source)? {
                        if property.enumerable {
                            let value = self.object_get(source.clone(), &key, span)?;
                            self.object_set(&object, &key, value, span)?;
                        }
                    }
                }
                Ok(object)
            }
            "create" => {
                if args.len() > 2 {
                    return Err("Object.create expects prototype and optional descriptors".into());
                }
                let parent = if let Some(value) = args.first() {
                    if matches!(value,Value::Json(node,_) if node.is_null()) {
                        None
                    } else {
                        self.object_own(value)?;
                        Some(value.clone())
                    }
                } else {
                    Some(self.object_root())
                };
                let object = new_object(parent);
                if let Some(descriptors) = args.get(1) {
                    for (key, property) in self.object_own(descriptors)? {
                        if let Some(descriptor) = property.value {
                            self.object_define(&object, &key, &descriptor)?;
                        }
                    }
                }
                Ok(object)
            }
            "construct" => {
                if args.is_empty() {
                    return Err("Object.construct needs a constructor".into());
                }
                let prototype = self.object_get(args[0].clone(), "prototype", span)?;
                let object = new_object(Some(prototype));
                let mut values = vec![object.clone()];
                values.extend(args.iter().skip(1).cloned());
                let result = self.object_invoke(args[0].clone(), values, span)?;
                Ok(if matches!(result, Value::PrototypeObject(..)) {
                    result
                } else {
                    object
                })
            }
            "assign" => {
                if args.is_empty() {
                    return Err("Object.assign needs a target".into());
                }
                self.object_state(&args[0])?;
                for source in args.iter().skip(1) {
                    for (key, property) in self.object_own(source)? {
                        if property.enumerable {
                            let value = self.object_get(source.clone(), &key, span)?;
                            self.object_set(&args[0], &key, value, span)?;
                        }
                    }
                }
                Ok(args[0].clone())
            }
            "defineProperty" => {
                self.object_define(&args[0], args[1].string()?, &args[2])?;
                Ok(args[0].clone())
            }
            "defineProperties" => {
                let descriptors = self.object_own(&args[1])?;
                for (key, property) in descriptors {
                    if let Some(value) = property.value {
                        self.object_define(&args[0], &key, &value)?;
                    }
                }
                Ok(args[0].clone())
            }
            "delete" => {
                let state = self.object_state(&args[0])?;
                let _world=prototype_read();
        let mut state = state.lock().map_err(|_| "object lock poisoned")?;
                let key = args[1].string()?;
                if state.properties.get(key).is_some_and(|p| !p.configurable) {
                    return Ok(Value::Bool(false));
                }
                let retired = state.properties.remove(key);
                drop(state);
                drop(_world);
                drop(retired);
                Ok(Value::Bool(true))
            }
            "freeze" | "seal" | "preventExtensions" => {
                let state = self.object_state(&args[0])?;
                let _world=prototype_read();
        let mut state = state.lock().map_err(|_| "object lock poisoned")?;
                state.extensible = false;
                if name != "preventExtensions" {
                    for property in state.properties.values_mut() {
                        property.configurable = false;
                        if name == "freeze" && property.value.is_some() {
                            property.writable = false;
                        }
                    }
                }
                Ok(args[0].clone())
            }
            "isExtensible" | "isSealed" | "isFrozen" => {
                let state = self.object_state(&args[0])?;
                let state = state.lock().map_err(|_| "object lock poisoned")?;
                Ok(Value::Bool(match name {
                    "isExtensible" => state.extensible,
                    "isSealed" => {
                        !state.extensible && state.properties.values().all(|p| !p.configurable)
                    }
                    _ => {
                        !state.extensible
                            && state
                                .properties
                                .values()
                                .all(|p| !p.configurable && (p.value.is_none() || !p.writable))
                    }
                }))
            }
            "keys" | "getOwnPropertyNames" | "values" | "entries" => {
                let own = self.object_own(&args[0])?;
                let mut values = Vec::new();
                for (key, property) in own {
                    if name != "getOwnPropertyNames" && !property.enumerable {
                        continue;
                    }
                    let value = match name {
                        "keys" | "getOwnPropertyNames" => Value::Str(key),
                        "values" => {
                            let v = self.object_get(args[0].clone(), &key, span)?;
                            self.object_box(v)?
                        }
                        _ => {
                            let v = self.object_get(args[0].clone(), &key, span)?;
                            let key = self.object_box(Value::Str(key))?;
                            let value = self.object_box(v)?;
                            Value::PrototypeArray(Arc::new(vec![key, value]), object_type())
                        }
                    };
                    values.push(value);
                }
                let ty = if name == "keys" || name == "getOwnPropertyNames" {
                    Type::Str
                } else {
                    object_type()
                };
                Ok(Value::Vector(Arc::new(values), Type::Vector(Box::new(ty))))
            }
            "getOwnPropertyDescriptor" => {
                let own = self.object_own(&args[0])?;
                if let Some(property) = own.get(args[1].string()?) {
                    self.descriptor_value(property.clone())
                } else {
                    Ok(Value::Json(
                        Arc::new(serde_json::Value::Null),
                        object_type(),
                    ))
                }
            }
            "getOwnPropertyDescriptors" => {
                let object = new_object(None);
                for (key, property) in self.object_own(&args[0])? {
                    let value = self.descriptor_value(property)?;
                    self.object_set(&object, &key, value, span)?;
                }
                Ok(object)
            }
            "fromEntries" => {
                let root = self.object_root();
                let object = new_object(Some(root));
                for entry in object_sequence(&args[0])? {
                    let pair = object_sequence(&entry)?;
                    if pair.len() != 2 {
                        return Err("fromEntries requires key/value pairs".into());
                    }
                    let key = match &pair[0] {
                        Value::Str(s) => s.clone(),
                        Value::Json(node, _) => node
                            .as_str()
                            .ok_or("fromEntries key must be str")?
                            .to_string(),
                        _ => return Err("fromEntries key must be str".into()),
                    };
                    let value = match &pair[1] {
                        Value::Json(node, ty) => {
                            crate::json::unwrap(node.as_ref().clone(), ty.clone())
                        }
                        value => value.clone(),
                    };
                    self.object_set(&object, &key, value, span)?;
                }
                Ok(object)
            }
            "groupBy" => {
                let object = new_object(None);
                let items = object_sequence(&args[0])?;
                let callable = match &args[1] {
                    Value::PrototypeFunction(v, _) => v.as_ref(),
                    v => v,
                };
                let Type::Function(params, ret) = callable.ty() else {
                    return Err("groupBy requires a Dev function".into());
                };
                if !(params.len() == 1 || (params.len() == 2 && params[1] == Type::i64()))
                    || *ret != Type::Str
                {
                    return Err(
                        "groupBy callback must return str and accept item plus optional index: i64"
                            .into(),
                    );
                }
                let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
                for (i, item) in items.into_iter().enumerate() {
                    let mut values = vec![item.clone()];
                    if params.len() == 2 {
                        values.push(Value::Int(i as i128, Type::i64()));
                    }
                    let key = self
                        .object_invoke(args[1].clone(), values, span)?
                        .string()?
                        .to_string();
                    groups.entry(key).or_default().push(self.object_box(item)?);
                }
                for (key, values) in groups {
                    self.object_set(
                        &object,
                        &key,
                        Value::PrototypeArray(Arc::new(values), object_type()),
                        span,
                    )?;
                }
                Ok(object)
            }
            "toString" => self.object_builtin(&["toString".into()], args, span),
            "valueOf" => Ok(args[0].clone()),
            _ => Err("unknown Object operation".into()),
        }
    }
}

fn object_same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::PrototypeFunction(a, _), b) => object_same(a, b),
        (a, Value::PrototypeFunction(b, _)) => object_same(a, b),
        (Value::PrototypeObject(a, _), Value::PrototypeObject(b, _)) => Arc::ptr_eq(a, b),
        (Value::Callable(am, an, ac, _), Value::Callable(bm, bn, bc, _)) => {
            am == bm
                && an == bn
                && match (ac, bc) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                    _ => false,
                }
        }
        (Value::Int(a, at), Value::Int(b, bt)) => a == b && at == bt,
        (Value::Float(a, at), Value::Float(b, bt)) => {
            at == bt && (a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan()))
        }
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        (Value::Json(a, _), Value::Json(b, _)) => Arc::ptr_eq(a, b) || (a.is_null() && b.is_null()),
        _ => false,
    }
}
impl Engine {
    fn object_json(
        &mut self,
        value: &Value,
        depth: usize,
        span: Span,
    ) -> Result<serde_json::Value, String> {
        if depth > 128 {
            return Err(
                "object serialization nesting exceeds 128; cycles are not serializable".into(),
            );
        }
        match value {
            Value::PrototypeObject(..) => {
                let own = self.object_own(value)?;
                let mut out = serde_json::Map::new();
                for (key, property) in own {
                    if property.enumerable {
                        let value = self.object_get(value.clone(), &key, span)?;
                        if !matches!(value, Value::Callable(..) | Value::PrototypeFunction(..)) {
                            out.insert(key, self.object_json(&value, depth + 1, span)?);
                        }
                    }
                }
                Ok(serde_json::Value::Object(out))
            }
            Value::PrototypeFunction(..) => Ok(serde_json::Value::Null),
            Value::PrototypeArray(items, _) => Ok(serde_json::Value::Array(
                items
                    .iter()
                    .map(|v| self.object_json(v, depth + 1, span))
                    .collect::<Result<_, _>>()?,
            )),
            Value::Array(items, _) => Ok(serde_json::Value::Array(
                items
                    .iter()
                    .map(|v| self.object_json(v, depth + 1, span))
                    .collect::<Result<_, _>>()?,
            )),
            Value::Vector(items, _) => Ok(serde_json::Value::Array(
                items
                    .iter()
                    .map(|v| self.object_json(v, depth + 1, span))
                    .collect::<Result<_, _>>()?,
            )),
            Value::Slice(items, o, n, _) => Ok(serde_json::Value::Array(
                items[*o..*o + *n]
                    .iter()
                    .map(|v| self.object_json(v, depth + 1, span))
                    .collect::<Result<_, _>>()?,
            )),
            _ => crate::json::encode(value, depth),
        }
    }
}

fn object_hint(value: Value, hint: Option<&Type>) -> Result<Value, String> {
    if let Some(t) = hint {
        convert(value, t, false)
    } else {
        Ok(value)
    }
}

impl Engine {
    fn array_argument(
        &mut self,
        f: &mut Frame,
        arg: &Expr,
        element: &Type,
    ) -> Result<Value, String> {
        if matches!(element,Type::Record(n,_)|Type::Nominal(n) if n=="dev_dynamic_value"||n=="dev_dynamic_don_value")
        {
            let v = self.object_argument(f, arg)?;
            let v = self.object_box(v)?;
            convert(v, element, false)
        } else {
            self.eval(f, arg, Some(element))
        }
    }
}
