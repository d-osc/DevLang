// Cycle collection uses Arc counts as external roots. No stack scanning or raw
// pointers are needed; graph mutations are excluded during a collection.
use std::sync::atomic::{AtomicUsize as GcAtomicUsize, Ordering as GcOrdering};
use std::sync::{OnceLock, RwLock, Weak};
static PROTOTYPE_COLLECTION_LOCK: Mutex<()> = Mutex::new(());
static PROTOTYPE_WORLD: RwLock<()> = RwLock::new(());
static PROTOTYPE_REGISTRY: OnceLock<Mutex<Vec<Weak<Mutex<ProtoObject>>>>> = OnceLock::new();
static PROTOTYPE_PENDING: GcAtomicUsize = GcAtomicUsize::new(0);
static PROTOTYPE_COLLECTIONS: GcAtomicUsize = GcAtomicUsize::new(0);
fn prototype_registry() -> &'static Mutex<Vec<Weak<Mutex<ProtoObject>>>> {
    PROTOTYPE_REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}
fn prototype_read() -> std::sync::RwLockReadGuard<'static, ()> {
    PROTOTYPE_WORLD.read().unwrap_or_else(|p| p.into_inner())
}
fn prototype_register(object: &Arc<Mutex<ProtoObject>>) {
    let _world = prototype_read();
    prototype_registry()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(Arc::downgrade(object));
    PROTOTYPE_PENDING.fetch_add(1, GcOrdering::Relaxed);
}
#[derive(Clone)]
enum GcNode {
    Object(Arc<Mutex<ProtoObject>>),
    Vector(Arc<Vec<Value>>),
    Reference(Arc<Value>),
    Map(Arc<MapStorage>),
}
impl GcNode {
    fn key(&self) -> (u8, usize) {
        match self {
            Self::Object(x) => (0, Arc::as_ptr(x) as usize),
            Self::Vector(x) => (1, Arc::as_ptr(x) as usize),
            Self::Reference(x) => (2, Arc::as_ptr(x) as usize),
            Self::Map(x) => (3, Arc::as_ptr(x) as usize),
        }
    }
    fn strong(&self) -> usize {
        match self {
            Self::Object(x) => Arc::strong_count(x),
            Self::Vector(x) => Arc::strong_count(x),
            Self::Reference(x) => Arc::strong_count(x),
            Self::Map(x) => Arc::strong_count(x),
        }
    }
}
#[derive(Default)]
struct GcGraph {
    nodes: Vec<GcNode>,
    ids: HashMap<(u8, usize), usize>,
    edges: Vec<Vec<usize>>,
    incoming: Vec<usize>,
    registry_roots: Vec<usize>,
}
impl GcGraph {
    fn node(&mut self, node: GcNode) -> usize {
        let key = node.key();
        if let Some(i) = self.ids.get(&key) {
            return *i;
        }
        let i = self.nodes.len();
        self.ids.insert(key, i);
        self.nodes.push(node);
        self.edges.push(Vec::new());
        self.incoming.push(0);
        self.registry_roots.push(0);
        i
    }
    fn edge(&mut self, from: usize, node: GcNode) {
        let to = self.node(node);
        self.edges[from].push(to);
        self.incoming[to] += 1;
    }
    fn value(&mut self, from: usize, v: &Value) {
        match v {
            Value::PrototypeObject(x, _) => self.edge(from, GcNode::Object(x.clone())),
            Value::PrototypeArray(x, _) | Value::Vector(x, _) | Value::Slice(x, _, _, _) => {
                self.edge(from, GcNode::Vector(x.clone()))
            }
            Value::Ref(x, _) => self.edge(from, GcNode::Reference(x.clone())),
            Value::Map(x, _) => self.edge(from, GcNode::Map(x.clone())),
            Value::Callable(_, _, Some(x), _) => self.edge(from, GcNode::Reference(x.clone())),
            Value::PrototypeFunction(x, _) => self.value(from, x),
            Value::Array(xs, _) | Value::Enum(_, _, xs) => {
                for x in xs {
                    self.value(from, x)
                }
            }
            Value::Record(xs, _) => {
                for (_, x) in xs {
                    self.value(from, x)
                }
            }
            // Foreign resources and unfinished tasks remain conservative external roots.
            _ => {}
        }
    }
    fn discover(&mut self) {
        let mut i = 0;
        while i < self.nodes.len() {
            let node = self.nodes[i].clone();
            match node {
                GcNode::Object(x) => {
                    let s = x.lock().unwrap_or_else(|p| p.into_inner());
                    if let Some(v) = &s.parent {
                        self.value(i, v)
                    }
                    for prop in s.properties.values() {
                        for v in [&prop.value, &prop.get, &prop.set].into_iter().flatten() {
                            self.value(i, v)
                        }
                    }
                }
                GcNode::Vector(xs) => {
                    for x in xs.iter() {
                        self.value(i, x)
                    }
                }
                GcNode::Reference(x) => self.value(i, &x),
                GcNode::Map(xs) => {
                    for (k, v) in xs.entries() {
                        self.value(i, k);
                        self.value(i, v)
                    }
                }
            }
            i += 1;
        }
    }
}
impl Engine {
    pub(crate) fn collect_prototype_cycles(&mut self, force: bool) -> usize {
        if !force && PROTOTYPE_PENDING.load(GcOrdering::Relaxed) < 256 {
            return 0;
        }
        let _collection = match PROTOTYPE_COLLECTION_LOCK.try_lock() {
            Ok(guard) => guard,
            Err(std::sync::TryLockError::Poisoned(p)) => p.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return 0,
        };
        let _world = PROTOTYPE_WORLD.write().unwrap_or_else(|p| p.into_inner());
        let objects = {
            let mut registry = prototype_registry()
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            registry.retain(|w| w.strong_count() > 0);
            registry
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>()
        };
        let mut graph = GcGraph::default();
        for object in objects {
            graph.node(GcNode::Object(object));
        }
        // Capturing functions keep metadata only while their context is reachable.
        // Global non-capturing function metadata remains a normal external root.
        for (key, owner) in &self.function_contexts {
            if let Some(state) = self.function_objects.get(key) {
                let meta = graph.node(GcNode::Object(state.clone()));
                graph.registry_roots[meta] += 1;
                if let Some(owner) = owner.upgrade() {
                    let context = graph.node(GcNode::Reference(owner));
                    graph.edges[context].push(meta);
                }
            }
        }
        graph.discover();
        let mut marked = vec![false; graph.nodes.len()];
        let mut todo = Vec::new();
        for (i, node) in graph.nodes.iter().enumerate() {
            if node.strong() > 1 + graph.incoming[i] + graph.registry_roots[i] {
                marked[i] = true;
                todo.push(i);
            }
        }
        while let Some(i) = todo.pop() {
            for &j in &graph.edges[i] {
                if !marked[j] {
                    marked[j] = true;
                    todo.push(j);
                }
            }
        }
        let mut collected = 0;
        let mut retired = Vec::new();
        for (i, node) in graph.nodes.iter().enumerate() {
            if !marked[i] {
                if let GcNode::Object(x) = node {
                    let mut s = x.lock().unwrap_or_else(|p| p.into_inner());
                    retired.push((std::mem::take(&mut s.properties), s.parent.take()));
                    collected += 1;
                }
            }
        }
        self.function_objects.retain(|key, state| {
            !self.function_contexts.contains_key(key)
                || graph
                    .ids
                    .get(&(0, Arc::as_ptr(state) as usize))
                    .is_some_and(|i| marked[*i])
        });
        self.function_contexts
            .retain(|key, _| self.function_objects.contains_key(key));
        PROTOTYPE_PENDING.store(0, GcOrdering::Relaxed);
        PROTOTYPE_COLLECTIONS.fetch_add(1, GcOrdering::Relaxed);
        drop(_world);
        // Drop resource-bearing values after releasing the graph mutation barrier.
        drop(graph);
        drop(retired);
        prototype_registry()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .retain(|w| w.strong_count() > 0);
        collected
    }
    fn gc_api(&mut self, name: &str) -> Result<Value, String> {
        let n = match name {
            "collect" => self.collect_prototype_cycles(true),
            "tracked" => prototype_registry()
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .iter()
                .filter(|w| w.strong_count() > 0)
                .count(),
            "collections" => PROTOTYPE_COLLECTIONS.load(GcOrdering::Relaxed),
            _ => return Err("unknown GC method".into()),
        };
        Ok(Value::Int(n as i128, Type::Size { signed: false }))
    }
}

#[cfg(test)]
mod gc_tests {
    use super::*;
    static TEST_GC_LOCK: Mutex<()> = Mutex::new(());
    fn engine() -> Engine {
        Engine::load_with_modules(
            Path::new("gc-test.dev"),
            Some("fn main(){}\nmain()\n"),
            vec![],
            &[],
            false,
            HashMap::new(),
        )
        .unwrap()
    }
    fn weak(value: &Value) -> Weak<Mutex<ProtoObject>> {
        let Value::PrototypeObject(state, _) = value else {
            panic!("object")
        };
        Arc::downgrade(state)
    }
    #[test]
    fn self_cycle_is_freed() {
        let _serial = TEST_GC_LOCK.lock().unwrap();
        let mut e = engine();
        let a = new_object(None);
        let w = weak(&a);
        e.object_set(&a, "self", a.clone(), Span::default())
            .unwrap();
        drop(a);
        e.collect_prototype_cycles(true);
        assert!(w.upgrade().is_none());
    }
    #[test]
    fn shared_container_preserves_live_children() {
        let _serial = TEST_GC_LOCK.lock().unwrap();
        let mut e = engine();
        let a = new_object(None);
        let b = new_object(None);
        let child = new_object(None);
        let wa = weak(&a);
        let wb = weak(&b);
        let wc = weak(&child);
        e.object_set(&child, "n", Value::Int(7, Type::i64()), Span::default())
            .unwrap();
        let xs = Arc::new(vec![child]);
        for object in [&a, &b] {
            e.object_set(object, "self", object.clone(), Span::default())
                .unwrap();
            e.object_set(
                object,
                "xs",
                Value::PrototypeArray(xs.clone(), object_type()),
                Span::default(),
            )
            .unwrap();
        }
        drop(a);
        drop(b);
        e.collect_prototype_cycles(true);
        assert!(wa.upgrade().is_none());
        assert!(wb.upgrade().is_none());
        assert!(wc.upgrade().is_some());
        assert_eq!(
            e.object_get(xs[0].clone(), "n", Span::default())
                .unwrap()
                .integer()
                .unwrap(),
            7
        );
        drop(xs);
        e.collect_prototype_cycles(true);
        assert!(wc.upgrade().is_none());
    }
    #[test]
    fn reference_and_map_cycles_are_freed() {
        let _serial = TEST_GC_LOCK.lock().unwrap();
        let mut e = engine();
        let a = new_object(None);
        let wa = weak(&a);
        let r = Arc::new(Value::Record(vec![("a".into(), a.clone())], object_type()));
        let mut map = MapStorage::default();
        map.entries.push((Value::Str("a".into()), a.clone()));
        let map = Arc::new(map);
        e.object_set(
            &a,
            "r",
            Value::Ref(r, Type::Ref(Box::new(object_type()))),
            Span::default(),
        )
        .unwrap();
        e.object_set(
            &a,
            "map",
            Value::Map(map, Type::Map(Box::new(Type::Str), Box::new(object_type()))),
            Span::default(),
        )
        .unwrap();
        drop(a);
        e.collect_prototype_cycles(true);
        assert!(wa.upgrade().is_none());
    }
}
