use crate::ast::*;
use crate::codegen::{c_type, Emitter, Value};
impl Emitter<'_> {
    pub(crate) fn resolve_collection_element(&self, t: &Type) -> Type {
        if let Type::Nominal(n) = t {
            self.program
                .modules
                .iter()
                .flat_map(|m| &m.concrete_types)
                .find(|t| t.name() == *n)
                .unwrap()
                .clone()
        } else {
            t.clone()
        }
    }
    pub(crate) fn collection(&mut self, t: &Type) -> String {
        self.uses_refs = true;
        self.uses_checks = true;
        let name = c_type(t);
        self.collection_types.insert(name.clone(), t.clone());
        if let Type::Map(k, v) = t {
            let entries = Type::Vector(Box::new(map_entry(t)));
            self.collection(&entries);
            self.collection(&Type::Vector(Box::new(Type::Size { signed: false })));
            self.helper(k, true);
            self.helper(k, false);
            let value = self.resolve_collection_element(v);
            self.helper(&value, true);
            self.helper(&value, false);
            return name;
        }
        let (Type::Vector(inner) | Type::Slice(inner)) = t else {
            unreachable!()
        };
        let element = self.resolve_collection_element(inner);
        self.helper(&element, true);
        self.helper(&element, false);
        name
    }
    pub(crate) fn vector(&mut self, t: &Type, args: &[Expr], span: Span) -> Result<Value, String> {
        if !self.hosted {
            return self.fail(span, "collections require hosted mode");
        }
        let Type::Vector(inner) = t else {
            unreachable!()
        };
        let element = self.resolve_collection_element(inner);
        let name = self.collection(t);
        let mut values = vec![];
        for arg in args {
            let v = self.expr(arg, Some(&element))?;
            self.compatible(&v, &element, arg.span)?;
            values.push(v.code);
        }
        let input = if values.is_empty() {
            "NULL".into()
        } else {
            format!("({}[]){{{}}}", c_type(&element), values.join(","))
        };
        Ok(self.owned(Value {
            code: format!("{name}_make({input}, {})", args.len()),
            ty: t.clone(),
            lvalue: false,
        }))
    }
    pub(crate) fn collection_call(
        &mut self,
        base: &Expr,
        op: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, String> {
        let previous = self.writing;
        self.writing = ["push", "pop", "clear", "set", "remove"].contains(&op);
        let v = self.expr(base, None)?;
        self.writing = previous;
        if matches!(v.ty, Type::Map(..)) {
            return self.map_call(v, op, args, span);
        }
        let (Type::Vector(inner) | Type::Slice(inner)) = &v.ty else {
            return self.fail(span, "collection method requires Vec or Slice");
        };
        let element = self.resolve_collection_element(inner);
        let name = self.collection(&v.ty);
        let count = match op {
            "push" => 1,
            "slice" => 2,
            "len" | "pop" | "clear" => 0,
            _ => return self.fail(span, "unknown collection method"),
        };
        if args.len() != count {
            return self.fail(span, "wrong collection argument count");
        }
        let location = self.location(span);
        match op {
            "len" => Ok(Value {
                code: format!("({}).len", v.code),
                ty: Type::Size { signed: false },
                lvalue: false,
            }),
            "slice" => {
                let mut indices = vec![];
                for a in args {
                    let i = self.expr(a, None)?;
                    if !i.ty.integer() {
                        return self.fail(a.span, "slice index requires integer");
                    };
                    indices.push(i.code);
                }
                let ty = Type::Slice(inner.clone());
                let slice = self.collection(&ty);
                Ok(self.owned(Value{code:format!("{slice}_slice(({}){{({}).len, ({}).offset, ({}).data}}, (uint64_t)({}), (uint64_t)({}), {location})",c_type(&ty),v.code,v.code,v.code,indices[0],indices[1]),ty,lvalue:false}))
            }
            _ => {
                if !v.lvalue || !matches!(v.ty, Type::Vector(_)) {
                    return self.fail(span, "mutable collection method requires a writable Vec");
                }
                if op == "push" {
                    let x = self.expr(&args[0], Some(&element))?;
                    self.compatible(&x, &element, args[0].span)?;
                    Ok(Value {
                        code: format!("{name}_push(&({}), {})", v.code, x.code),
                        ty: Type::Void,
                        lvalue: false,
                    })
                } else if op == "clear" {
                    Ok(Value {
                        code: format!("{name}_clear(&({}))", v.code),
                        ty: Type::Void,
                        lvalue: false,
                    })
                } else {
                    Ok(self.owned(Value {
                        code: format!("{name}_pop(&({}), {location})", v.code),
                        ty: element,
                        lvalue: false,
                    }))
                }
            }
        }
    }
    pub(crate) fn collection_typedefs(&self) -> String {
        let mut out = String::new();
        let mut keys = self.collection_types.keys().collect::<Vec<_>>();
        keys.sort();
        for name in keys {
            if matches!(self.collection_types[name], Type::Map(..)) {
                continue;
            }
            let (Type::Vector(t) | Type::Slice(t)) = &self.collection_types[name] else {
                unreachable!()
            };
            out.push_str(&format!("#ifndef {name}_DEFINED\n#define {name}_DEFINED\ntypedef struct {{ size_t len;size_t offset;{} *data; }} {name};\n#endif\n",c_type(t)));
        }
        out
    }
    pub(crate) fn collection_definitions(&mut self) -> String {
        let mut out = String::new();
        let mut types = self.collection_types.values().cloned().collect::<Vec<_>>();
        types.sort_by_key(c_type);
        for t in types
            .iter()
            .filter(|t| !matches!(t, Type::Map(..)))
            .cloned()
        {
            let name = c_type(&t);
            let (Type::Vector(inner) | Type::Slice(inner)) = &t else {
                unreachable!()
            };
            let element = self.resolve_collection_element(inner);
            let ct = c_type(&element);
            let keep = self.helper(&element, true);
            let drop = self.helper(&element, false);
            out.push_str(&format!("static {ct} const *{name}_at(const {name} *v,uint64_t i,const char *p,size_t l,size_t c) {{ if(i>=v->len) dev_checked_fail(p,l,c,\"array index out of bounds\");return &v->data[v->offset+i]; }}\n"));
            if matches!(t, Type::Slice(_)) {
                out.push_str(&format!("static {name} {name}_slice({name} v,uint64_t a,uint64_t b,const char *p,size_t l,size_t c) {{ if(a>b || b>v.len) dev_checked_fail(p,l,c,\"slice range out of bounds\");dev_ref_retain(v.data);v.offset+=(size_t)a;v.len=(size_t)(b-a);return v; }}\n"));
                continue;
            }
            out.push_str(&format!("static void {name}_drop(void *data) {{ struct dev_ref_block *b=((struct dev_ref_block *)data)-1;{ct} *xs=({ct} *)data;for(size_t i=0;i<b->items;++i) {drop}(&xs[i]); }}\n"));
            out.push_str(&format!("static {name} {name}_make({ct} const *input,size_t n) {{ size_t cap=n<8?8:n;if(cap>(SIZE_MAX-sizeof(struct dev_ref_block))/sizeof({ct})) abort();struct dev_ref_block *b=(struct dev_ref_block *)malloc(sizeof(*b)+cap*sizeof({ct}));if(!b) abort();b->count=1;b->bytes=cap*sizeof({ct});b->items=n;b->drop={name}_drop;{ct} *xs=({ct} *)(b+1);for(size_t i=0;i<n;++i) {{xs[i]=input[i];{keep}(&xs[i]);}} {name} v={{n,0,xs}};return v; }}\n"));
            out.push_str(&format!("static void {name}_unique({name} *v) {{ if(!v->data) {{*v={name}_make(NULL,0);return;}}struct dev_ref_block *b=((struct dev_ref_block *)v->data)-1;if(b->count>1) {{ {name} copy={name}_make(v->data+v->offset,v->len);dev_ref_release(v->data);*v=copy; }} }}\n"));
            out.push_str(&format!("static {ct} *{name}_at_mut({name} *v,uint64_t i,const char *p,size_t l,size_t c) {{ if(i>=v->len) dev_checked_fail(p,l,c,\"array index out of bounds\");{name}_unique(v);return &v->data[i]; }}\n"));
            out.push_str(&format!("static void {name}_push({name} *v,{ct} value) {{ {keep}(&value); {name}_unique(v);struct dev_ref_block *b=((struct dev_ref_block *)v->data)-1;size_t cap=b->bytes/sizeof({ct});if(v->len==cap){{if(cap>(SIZE_MAX-sizeof(*b))/(2*sizeof({ct}))) abort();cap*=2;struct dev_ref_block *grown=(struct dev_ref_block *)realloc(b,sizeof(*b)+cap*sizeof({ct}));if(!grown) abort();b=grown;b->bytes=cap*sizeof({ct});v->data=({ct} *)(b+1);}}v->data[v->len]=value;++v->len;b->items=v->len; }}\n"));
            out.push_str(&format!("static {ct} {name}_pop({name} *v,const char *p,size_t l,size_t c) {{ if(!v->len) dev_checked_fail(p,l,c,\"pop from empty Vec\");{name}_unique(v);{ct} value=v->data[--v->len];(((struct dev_ref_block *)v->data)-1)->items=v->len;return value; }}\nstatic void {name}_clear({name} *v) {{dev_ref_release(v->data);v->data=NULL;v->len=0;v->offset=0;}}\n"));
        }
        for t in types.iter().filter(|t| matches!(t, Type::Map(..))) {
            out.push_str(&self.map_definitions(t));
        }
        out
    }
}

impl Emitter<'_> {
    pub(crate) fn map_constructor(&mut self, t: &Type, span: Span) -> Result<Value, String> {
        if !self.hosted {
            return self.fail(span, "collections require hosted mode");
        }
        self.collection(t);
        Ok(self.owned(Value {
            code: format!("(({}){{0}})", c_type(t)),
            ty: t.clone(),
            lvalue: false,
        }))
    }
    fn map_call(&mut self, v: Value, op: &str, args: &[Expr], span: Span) -> Result<Value, String> {
        let Type::Map(k, value) = &v.ty else {
            unreachable!()
        };
        let value = self.resolve_collection_element(value);
        let name = self.collection(&v.ty);
        let count = match op {
            "len" | "clear" => 0,
            "set" => 2,
            "get" | "contains" | "remove" => 1,
            _ => return self.fail(span, "unknown Map method"),
        };
        if args.len() != count {
            return self.fail(span, "wrong collection argument count");
        }
        if ["set", "remove", "clear"].contains(&op) && !v.lvalue {
            return self.fail(span, "mutable Map method requires writable Map");
        }
        let mut values = vec![];
        for (i, arg) in args.iter().enumerate() {
            let t = if i == 0 { k.as_ref() } else { &value };
            let x = self.expr(arg, Some(t))?;
            self.compatible(&x, t, arg.span)?;
            values.push(x.code);
        }
        let location = self.location(span);
        let (code, ret) = match op {
            "len" => (
                format!("({}).entries.len", v.code),
                Type::Size { signed: false },
            ),
            "clear" => (format!("{name}_clear(&({}))", v.code), Type::Void),
            "contains" => (
                format!("({name}_find(&({}),{})!=(size_t)-1)", v.code, values[0]),
                Type::Bool,
            ),
            "get" => (
                format!("{name}_get(&({}),{}, {location})", v.code, values[0]),
                value,
            ),
            "set" => (
                format!("{name}_set(&({}),{}, {})", v.code, values[0], values[1]),
                Type::Void,
            ),
            "remove" => (
                format!("{name}_remove(&({}),{})", v.code, values[0]),
                Type::Bool,
            ),
            _ => unreachable!(),
        };
        let result = Value {
            code,
            ty: ret,
            lvalue: false,
        };
        Ok(if op == "get" {
            self.owned(result)
        } else {
            result
        })
    }
    fn map_definitions(&mut self, t: &Type) -> String {
        let Type::Map(k, v) = t else { unreachable!() };
        let name = c_type(t);
        let entry = map_entry(t);
        let entry_ct = c_type(&entry);
        let vector = c_type(&Type::Vector(Box::new(entry.clone())));
        let value = self.resolve_collection_element(v);
        let vt = c_type(&value);
        let kt = c_type(k);
        let keep = self.helper(&value, true);
        let drop = self.helper(&value, false);
        let drop_entry = self.helper(&entry, false);
        let compare = if **k == Type::Str {
            "strcmp(a,b)==0".into()
        } else {
            "a==b".to_string()
        };
        let buckets = c_type(&Type::Vector(Box::new(Type::Size { signed: false })));
        let hash = match k.as_ref() {
            Type::Str => "uint64_t h=14695981039346656037ULL;for(const unsigned char *p=(const unsigned char *)key;*p;++p){h^=*p;h*=1099511628211ULL;}".into(),
            Type::Float(bits) => {
                let uint = if *bits == 32 { "uint32_t" } else { "uint64_t" };
                format!("{uint} raw=0;if(key!=0)memcpy(&raw,&key,sizeof(raw));uint64_t h=raw;")
            }
            _ => "uint64_t h=(uint64_t)key;".into(),
        };
        let out = format!(
            r#"
static bool {name}_equal({kt} a,{kt} b) {{ return {compare}; }}
static size_t {name}_hash({kt} key) {{ {hash} h^=h>>30;h*=0xbf58476d1ce4e5b9ULL;h^=h>>27;h*=0x94d049bb133111ebULL;h^=h>>31;return (size_t)h; }}
static size_t {name}_slot(const {name} *m,{kt} key) {{size_t mask=m->buckets.len-1,s={name}_hash(key)&mask;while(m->buckets.data[s] && !{name}_equal(m->entries.data[m->buckets.data[s]-1].dev_f_0,key))s=(s+1)&mask;return s;}}
static size_t {name}_find(const {name} *m,{kt} key) {{if(!m->buckets.len)return (size_t)-1;size_t n=m->buckets.data[{name}_slot(m,key)];return n?n-1:(size_t)-1;}}
static void {name}_rehash({name} *m,size_t capacity) {{ {buckets}_clear(&m->buckets);for(size_t i=0;i<capacity;++i){buckets}_push(&m->buckets,0);for(size_t i=0;i<m->entries.len;++i)m->buckets.data[{name}_slot(m,m->entries.data[i].dev_f_0)]=i+1; }}
static {vt} {name}_get(const {name} *m,{kt} key,const char *p,size_t l,size_t c) {{size_t i={name}_find(m,key);if(i==(size_t)-1)dev_checked_fail(p,l,c,"Map key not found");{vt} value=m->entries.data[i].dev_f_1;{keep}(&value);return value;}}
static void {name}_set({name} *m,{kt} key,{vt} value) {{
    {keep}(&value);size_t i={name}_find(m,key);
    if(i!=(size_t)-1){{{vector}_unique(&m->entries);{drop}(&m->entries.data[i].dev_f_1);m->entries.data[i].dev_f_1=value;}}
    else{{
        if(!m->buckets.len){name}_rehash(m,16);
        else if(m->entries.len>=m->buckets.len/2){{if(m->buckets.len>SIZE_MAX/2)abort();{name}_rehash(m,m->buckets.len*2);}}
        else {buckets}_unique(&m->buckets);
        {entry_ct} e={{key,value}};{vector}_push(&m->entries,e);{drop}(&value);
        m->buckets.data[{name}_slot(m,key)]=m->entries.len;
    }}
}}
static bool {name}_remove({name} *m,{kt} key) {{
    size_t i={name}_find(m,key);if(i==(size_t)-1)return false;
    {vector}_unique(&m->entries);{buckets}_unique(&m->buckets);
    size_t mask=m->buckets.len-1,hole={name}_slot(m,key),scan=(hole+1)&mask;
    while(m->buckets.data[scan]){{size_t home={name}_hash(m->entries.data[m->buckets.data[scan]-1].dev_f_0)&mask;
        if(((scan-home)&mask)>=((scan-hole)&mask)){{m->buckets.data[hole]=m->buckets.data[scan];hole=scan;}}scan=(scan+1)&mask;
    }}m->buckets.data[hole]=0;
    {drop_entry}(&m->entries.data[i]);size_t last=--m->entries.len;
    if(i!=last){{m->entries.data[i]=m->entries.data[last];size_t moved={name}_hash(m->entries.data[i].dev_f_0)&mask;while(m->buckets.data[moved]!=last+1)moved=(moved+1)&mask;m->buckets.data[moved]=i+1;}}
    (((struct dev_ref_block *)m->entries.data)-1)->items=m->entries.len;return true;
}}
static void {name}_clear({name} *m) {{ {vector}_clear(&m->entries);{buckets}_clear(&m->buckets); }}
"#
        );

        out
    }
}
