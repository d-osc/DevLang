use crate::ast::*;
use crate::codegen::{c_type, Emitter, Value};
pub const THREADS: &str = r#"
#include <stdlib.h>
#include <stdatomic.h>
#define DEV_COUNT _Atomic size_t
#if defined(_WIN32)
#include <windows.h>
typedef HANDLE dev_thread;
typedef CRITICAL_SECTION dev_mutex;
#define DEV_THREAD_ENTRY DWORD WINAPI
#define DEV_THREAD_EXIT return 0
static void dev_mutex_init(dev_mutex *m){InitializeCriticalSection(m);}
static void dev_mutex_lock(dev_mutex *m){EnterCriticalSection(m);}
static void dev_mutex_unlock(dev_mutex *m){LeaveCriticalSection(m);}
static void dev_mutex_destroy(dev_mutex *m){DeleteCriticalSection(m);}
static void dev_thread_start(dev_thread *t,DWORD (WINAPI *fn)(void *),void *p){*t=CreateThread(NULL,0,fn,p,0,NULL);if(!*t)abort();}
static void dev_thread_join(dev_thread t){if(WaitForSingleObject(t,INFINITE)!=WAIT_OBJECT_0)abort();CloseHandle(t);}
static void dev_thread_detach(dev_thread t){CloseHandle(t);}
#else
#include <pthread.h>
typedef pthread_t dev_thread;
typedef pthread_mutex_t dev_mutex;
#define DEV_THREAD_ENTRY void *
#define DEV_THREAD_EXIT return NULL
static void dev_mutex_init(dev_mutex *m){if(pthread_mutex_init(m,NULL))abort();}
static void dev_mutex_lock(dev_mutex *m){if(pthread_mutex_lock(m))abort();}
static void dev_mutex_unlock(dev_mutex *m){if(pthread_mutex_unlock(m))abort();}
static void dev_mutex_destroy(dev_mutex *m){if(pthread_mutex_destroy(m))abort();}
static void dev_thread_start(dev_thread *t,void *(*fn)(void *),void *p){if(pthread_create(t,NULL,fn,p))abort();}
static void dev_thread_join(dev_thread t){if(pthread_join(t,NULL))abort();}
static void dev_thread_detach(dev_thread t){if(pthread_detach(t))abort();}
#endif
"#;
impl Emitter<'_> {
    pub(crate) fn task_call(
        &mut self,
        op: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, String> {
        if !self.hosted {
            return self.fail(span, "tasks require hosted mode");
        }
        if args.len() != 1 {
            return self.fail(span, "spawn/await/ready expect one argument");
        }
        let v = self.expr(&args[0], None)?;
        let r = match (&v.ty, op) {
            (Type::Function(ps, r), "spawn") if ps.is_empty() => r.clone(),
            (Type::Task(r), "await" | "ready") => r.clone(),
            _ => return self.fail(span, "spawn expects fn() T; await/ready expect Task<T>"),
        };
        let task = Type::Task(r.clone());
        let name = c_type(&task);
        self.task_types.insert(name.clone(), task.clone());
        let callable = Type::Function(vec![], r.clone());
        self.helper(&callable, true);
        self.helper(&callable, false);
        if *r != Type::Void {
            self.helper(&r, true);
            self.helper(&r, false);
        }
        self.uses_refs = true;
        Ok(self.owned(Value {
            code: format!(
                "{name}_{}({})",
                match op {
                    "spawn" => "start",
                    "ready" => "ready",
                    _ => "wait",
                },
                v.code
            ),
            ty: match op {
                "spawn" => task,
                "ready" => Type::Bool,
                _ => *r,
            },
            lvalue: false,
        }))
    }
    pub(crate) fn task_definitions(&self) -> String {
        let mut types = self.task_types.iter().collect::<Vec<_>>();
        types.sort_by_key(|(n, _)| *n);
        let mut out = String::new();
        for (name, t) in types {
            let Type::Task(r) = t else { unreachable!() };
            let callable = Type::Function(vec![], r.clone());
            let job = c_type(&callable);
            let jobkey = crate::codegen::managed_key(&callable);
            let key = crate::codegen::managed_key(r);
            let field = if **r == Type::Void {
                "unsigned char result;".into()
            } else {
                format!("{} result;", c_type(r))
            };
            let compute: String = if **r == Type::Void {
                "p->job.call(p->job.context);".into()
            } else {
                "p->result=p->job.call(p->job.context);".into()
            };
            let drop = if **r == Type::Void {
                String::new()
            } else {
                format!("dev_drop_{key}(&p->result);")
            };
            let copy = if **r == Type::Void {
                String::new()
            } else {
                format!("{} result=p->result; dev_keep_{key}(&result);", c_type(r))
            };
            let returned = if **r == Type::Void {
                "return;"
            } else {
                "return result;"
            };
            out.push_str(&format!(r#"
struct {name}_state {{ dev_thread thread; dev_mutex mutex; int joined; atomic_bool done; {job} job; {field} }};
static DEV_THREAD_ENTRY {name}_entry(void *v) {{ struct {name}_state *p=v; {compute} atomic_store_explicit(&p->done,true,memory_order_release);dev_ref_release(p); DEV_THREAD_EXIT; }}
static void {name}_drop(void *v) {{ struct {name}_state *p=v; if(!p->joined) dev_thread_detach(p->thread); dev_drop_{jobkey}(&p->job); {drop} dev_mutex_destroy(&p->mutex); }}
static {name} {name}_start({job} job) {{ dev_keep_{jobkey}(&job); struct {name}_state initial={{0}};initial.job=job;struct {name}_state *p=dev_ref_copy(&initial,sizeof(initial),NULL,{name}_drop);dev_mutex_init(&p->mutex);atomic_init(&p->done,false);dev_ref_retain(p);dev_thread_start(&p->thread,{name}_entry,p);return ({name}){{p}}; }}
static bool {name}_ready({name} t) {{struct {name}_state *p=t.state;return atomic_load_explicit(&p->done,memory_order_acquire);}}
static {ret} {name}_wait({name} t) {{ struct {name}_state *p=t.state; dev_mutex_lock(&p->mutex);if(!p->joined){{dev_thread_join(p->thread);p->joined=1;}} {copy} dev_mutex_unlock(&p->mutex); {returned} }}
"#,ret=c_type(r)));
        }
        out
    }
}
