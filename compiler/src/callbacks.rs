use crate::ast::*;
use crate::codegen::{c_type, declaration, managed_key, Emitter};

impl Emitter<'_> {
    /// Portable C trampolines retain environments for process lifetime. Slots are
    /// never reused: a foreign copy of a callback pointer must retain its identity.
    pub(crate) fn callback_definitions(&self) -> String {
        let mut types = self.callback_types.iter().collect::<Vec<_>>();
        types.sort_by_key(|(name, _)| *name);
        let mut out = String::new();
        for (name, t) in types {
            let Type::Callback(ps, r) = t else {
                unreachable!()
            };
            let callable = Type::Function(ps.clone(), r.clone());
            let job = c_type(&callable);
            let keep = managed_key(&callable);
            let params = if ps.is_empty() {
                "void".into()
            } else {
                ps.iter()
                    .enumerate()
                    .map(|(i, t)| declaration(t, &format!("p{i}")))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let args = (0..ps.len()).map(|i| format!(",p{i}")).collect::<String>();
            out.push_str(&format!("static {job} {name}_jobs[64];\nstatic size_t {name}_count;\nstatic atomic_flag {name}_lock=ATOMIC_FLAG_INIT;\n"));
            for i in 0..64 {
                out.push_str(&format!("static {} {name}_slot_{i}({params}) {{ {}{name}_jobs[{i}].call({name}_jobs[{i}].context{args}); }}\n",c_type(r),if **r == Type::Void { "" } else { "return " }));
            }
            let slots = (0..64)
                .map(|i| format!("{name}_slot_{i}"))
                .collect::<Vec<_>>()
                .join(",");
            out.push_str(&format!(r#"
static {name} {name}_create({job} job,const char *p,size_t l,size_t c) {{
    static const {name} slots[64]={{{slots}}};
    while(atomic_flag_test_and_set_explicit(&{name}_lock,memory_order_acquire)){{}}
    for(size_t i=0;i<{name}_count;++i){{if({name}_jobs[i].call==job.call && {name}_jobs[i].context==job.context){{atomic_flag_clear_explicit(&{name}_lock,memory_order_release);return slots[i];}}}}
    if({name}_count==64){{atomic_flag_clear_explicit(&{name}_lock,memory_order_release);dev_checked_fail(p,l,c,"native capturing callback capacity exceeded (64 per signature per module)");}}
    size_t i={name}_count++;dev_keep_{keep}(&job);{name}_jobs[i]=job;
    atomic_flag_clear_explicit(&{name}_lock,memory_order_release);return slots[i];
}}
"#));
        }
        let mut contexts = self.context_callback_types.iter().collect::<Vec<_>>();
        contexts.sort_by_key(|(n, _)| *n);
        for (name, function) in contexts {
            let Type::Function(ps, r) = function else {
                unreachable!()
            };
            let job = c_type(function);
            let key = managed_key(function);
            let mut params = ps
                .iter()
                .enumerate()
                .map(|(i, t)| declaration(t, &format!("p{i}")))
                .collect::<Vec<_>>();
            params.push("void *data".into());
            let args = (0..ps.len()).map(|i| format!(",p{i}")).collect::<String>();
            let compute = if **r == Type::Void {
                format!("owner->call(owner->context{args});")
            } else {
                format!("{} result=owner->call(owner->context{args});", c_type(r))
            };
            let returned = if **r == Type::Void {
                "return;"
            } else {
                "return result;"
            };
            out.push_str(&format!(
                r#"
static {ret} {name}_bridge({params}) {{
    const {job} *owner=dev_checked_pointer((uintptr_t)data,_Alignof({job}),"context callback",0,0);
    dev_ref_retain(owner);{compute} dev_ref_release(owner);{returned}
}}
static {name} {name}_make({job} function) {{
    const {job} *owner=dev_ref_copy(&function,sizeof(function),dev_keep_{key},dev_drop_{key});
    return ({name}){{{name}_bridge,(void *)owner,owner}};
}}
"#,
                ret = c_type(r),
                params = params.join(",")
            ));
        }
        out
    }
}
