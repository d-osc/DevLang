/* Prototype state is shared by aliases; data arrays remain value snapshots. */
typedef struct dev_object_property {
  const char *key;
  dev_dynamic_value value, get, set;
  bool accessor, writable, enumerable, configurable;
} dev_object_property;
typedef struct dev_object_meta {
  atomic_flag lock;
  size_t len;
  dev_object_property *props;
  dev_dynamic_value parent;
  bool extensible;
} dev_object_meta;
typedef dev_dynamic_value (*dev_object_adapter)(const void *, size_t,
                                                const dev_dynamic_value *,
                                                const char *, size_t, size_t);
typedef struct dev_object_function {
  struct dev_object_function *next;
  dev_object_adapter invoke;
  void (*original)(void);
  const void *context;
  const char *identity;
  void *payload;
  int builtin, result_kind;
  size_t argc;
  const char *name;
  dev_dynamic_node *node;
  dev_object_meta meta;
} dev_object_function;
extern dev_dynamic_node *dev_object_global_root;
extern dev_object_function *dev_object_global_functions;
extern atomic_flag dev_object_global_lock;
static dev_dynamic_value dev_object_api(const char *, size_t,
                                        const dev_dynamic_value *, const char *,
                                        size_t, size_t);
static dev_dynamic_value dev_object_invoke(dev_dynamic_value, size_t,
                                           const dev_dynamic_value *,
                                           const char *, size_t, size_t);
static void dev_object_lock(atomic_flag *lock) {
  while (atomic_flag_test_and_set_explicit(lock, memory_order_acquire)) {
  }
}
static void dev_object_unlock(atomic_flag *lock) {
  atomic_flag_clear_explicit(lock, memory_order_release);
}
static dev_object_meta *dev_object_meta_of(dev_dynamic_value v, const char *p,
                                           size_t l, size_t c) {
  if (!v.node)
    dev_dynamic_error("operation requires a prototype Object", p, l, c);
  const dev_dynamic_node *n = dev_dynamic_node_of(v);
  if (n->kind == 9)
    return n->data.opaque;
  if (n->kind == 8)
    return &((dev_object_function *)n->data.opaque)->meta;
  dev_dynamic_error(
      "operation requires a prototype Object; construct with Object({...})", p,
      l, c);
  return NULL;
}
static bool dev_object_managed(dev_dynamic_value v) {
  return v.node && (dev_dynamic_node_of(v)->kind == 8 ||
                    dev_dynamic_node_of(v)->kind == 9);
}
static void dev_object_init(dev_object_meta *m, dev_dynamic_value parent) {
  atomic_flag_clear(&m->lock);
  m->parent = parent;
  m->extensible = true;
}
static dev_dynamic_value dev_object_new(dev_dynamic_value parent) {
  dev_dynamic_value v = dev_dynamic_make(9);
  dev_object_meta *m = dev_dynamic_alloc(sizeof(*m));
  dev_object_init(m, parent);
  ((dev_dynamic_node *)v.node)->data.opaque = m;
  return v;
}
static dev_object_property dev_object_data_prop(const char *key,
                                                dev_dynamic_value value) {
  return (dev_object_property){key, value, {0}, {0}, false, true, true, true};
}
static void dev_object_put_raw(dev_object_meta *m,
                               dev_object_property property) {
  size_t i = 0;
  while (i < m->len && strcmp(m->props[i].key, property.key) < 0)
    ++i;
  bool found = i < m->len && !strcmp(m->props[i].key, property.key);
  dev_object_property *next =
      dev_dynamic_alloc((m->len + !found) * sizeof(*next));
  if (i)
    memcpy(next, m->props, i * sizeof(*next));
  next[i] = property;
  if (found) {
    if (m->len - i - 1)
      memcpy(next + i + 1, m->props + i + 1, (m->len - i - 1) * sizeof(*next));
  } else {
    if (m->len - i)
      memcpy(next + i + 1, m->props + i, (m->len - i) * sizeof(*next));
    ++m->len;
  }
  m->props = next;
}
static bool dev_object_own_prop(dev_dynamic_value object, const char *key,
                                dev_object_property *out, const char *p,
                                size_t l, size_t c) {
  if (dev_object_managed(object)) {
    dev_object_meta *m = dev_object_meta_of(object, p, l, c);
    dev_object_lock(&m->lock);
    bool found = false;
    for (size_t i = 0; i < m->len; i++)
      if (!strcmp(m->props[i].key, key)) {
        *out = m->props[i];
        found = true;
        break;
      }
    dev_object_unlock(&m->lock);
    return found;
  }
  if (dev_dynamic_tag(object) != 7)
    dev_dynamic_error("expected object", p, l, c);
  const dev_dynamic_node *n = dev_dynamic_node_of(object);
  for (size_t i = 0; i < n->data.list.len; i++)
    if (!strcmp(n->data.list.keys[i], key)) {
      dev_dynamic_value value = n->data.list.values[i];
      if (dev_dynamic_tag(value) > 0 && dev_dynamic_tag(value) < 6)
        value = dev_dynamic_unboxed(value);
      *out = dev_object_data_prop(n->data.list.keys[i], value);
      return true;
    }
  return false;
}
static bool dev_object_lookup(dev_dynamic_value object, const char *key,
                              dev_object_property *out, const char *p, size_t l,
                              size_t c) {
  for (unsigned depth = 0; depth < 128; depth++) {
    if (dev_object_own_prop(object, key, out, p, l, c))
      return true;
    if (!dev_object_managed(object))
      return false;
    dev_object_meta *m = dev_object_meta_of(object, p, l, c);
    dev_object_lock(&m->lock);
    object = m->parent;
    dev_object_unlock(&m->lock);
    if (!object.node)
      return false;
  }
  dev_dynamic_error("prototype chain exceeds 128 levels", p, l, c);
  return false;
}
static dev_dynamic_value dev_object_get(dev_dynamic_value object,
                                        const char *key, const char *p,
                                        size_t l, size_t c) {
  dev_object_property property;
  if (!dev_object_lookup(object, key, &property, p, l, c))
    dev_dynamic_error("missing object property", p, l, c);
  if (property.accessor) {
    if (property.get.node)
      return dev_object_invoke(property.get, 1, &object, p, l, c);
    return (dev_dynamic_value){NULL, 1};
  }
  return property.value;
}
static void dev_object_set(dev_dynamic_value object, const char *key,
                           dev_dynamic_value value, const char *p, size_t l,
                           size_t c) {
  dev_object_meta *m = dev_object_meta_of(object, p, l, c);
  dev_object_property inherited;
  if (dev_object_lookup(object, key, &inherited, p, l, c)) {
    if (inherited.accessor) {
      if (!inherited.set.node)
        dev_dynamic_error("object property is read-only", p, l, c);
      dev_dynamic_value args[] = {object, value};
      (void)dev_object_invoke(inherited.set, 2, args, p, l, c);
      return;
    }
    if (!inherited.writable)
      dev_dynamic_error("object property is read-only", p, l, c);
  }
  dev_object_lock(&m->lock);
  dev_object_property current;
  bool found = false;
  for (size_t i = 0; i < m->len; i++)
    if (!strcmp(m->props[i].key, key)) {
      current = m->props[i];
      found = true;
      break;
    }
  if (found) {
    if (!current.writable) {
      dev_object_unlock(&m->lock);
      dev_dynamic_error("object property is read-only", p, l, c);
    }
    current.value = value;
    dev_object_put_raw(m, current);
  } else {
    if (!m->extensible) {
      dev_object_unlock(&m->lock);
      dev_dynamic_error("object is not extensible", p, l, c);
    }
    dev_object_put_raw(
        m, dev_object_data_prop(dev_dynamic_copy_string(key), value));
  }
  dev_object_unlock(&m->lock);
}
static dev_dynamic_value dev_object_builtin_node(int builtin, const char *name,
                                                 dev_dynamic_value root) {
  dev_dynamic_value v = dev_dynamic_make(8);
  dev_object_function *fn = dev_dynamic_alloc(sizeof(*fn));
  fn->builtin = builtin;
  fn->name = name;
  fn->node = (dev_dynamic_node *)v.node;
  dev_object_init(&fn->meta, root);
  fn->node->data.opaque = fn;
  dev_dynamic_value proto = builtin == 1 ? root : dev_object_new(root);
  dev_object_property property = dev_object_data_prop("prototype", proto);
  property.enumerable = false;
  property.configurable = false;
  if (builtin == 1)
    property.writable = false;
  dev_object_put_raw(&fn->meta, property);
  if (builtin != 1) {
    dev_object_meta *meta = dev_object_meta_of(proto, "<Object>", 0, 0);
    property = dev_object_data_prop("constructor", v);
    property.enumerable = false;
    dev_object_put_raw(meta, property);
  }
  return v;
}
static dev_dynamic_value dev_object_root(void) {
  dev_object_lock(&dev_object_global_lock);
  if (dev_object_global_root) {
    dev_dynamic_value v = {dev_object_global_root, 1};
    dev_object_unlock(&dev_object_global_lock);
    return v;
  }
  dev_dynamic_value root = dev_object_new((dev_dynamic_value){0});
  dev_object_global_root = (dev_dynamic_node *)root.node;
  dev_object_meta *m = dev_object_meta_of(root, "<Object>", 0, 0);
  dev_dynamic_value ctor = dev_object_builtin_node(1, "constructor", root);
  const char *methods[] = {"assign",
                           "create",
                           "defineProperties",
                           "defineProperty",
                           "delete",
                           "entries",
                           "freeze",
                           "fromEntries",
                           "getOwnPropertyDescriptor",
                           "getOwnPropertyDescriptors",
                           "getOwnPropertyNames",
                           "groupBy",
                           "isExtensible",
                           "isFrozen",
                           "isSealed",
                           "keys",
                           "preventExtensions",
                           "seal",
                           "toString",
                           "valueOf",
                           "values",
                           "construct"};
  dev_object_meta *constructor = dev_object_meta_of(ctor, "<Object>", 0, 0);
  for (size_t i = 0; i < sizeof(methods) / sizeof(*methods); i++) {
    dev_object_property prop = dev_object_data_prop(
        methods[i], dev_object_builtin_node(100, methods[i], root));
    prop.enumerable = false;
    dev_object_put_raw(constructor, prop);
  }
  dev_object_property prop = dev_object_data_prop("constructor", ctor);
  prop.enumerable = false;
  dev_object_put_raw(m, prop);
  prop = dev_object_data_prop("toString",
                              dev_object_builtin_node(2, "toString", root));
  prop.enumerable = false;
  dev_object_put_raw(m, prop);
  prop = dev_object_data_prop("valueOf",
                              dev_object_builtin_node(3, "valueOf", root));
  prop.enumerable = false;
  dev_object_put_raw(m, prop);
  dev_object_unlock(&dev_object_global_lock);
  return root;
}
static dev_dynamic_value
dev_object_function_value(dev_object_adapter invoke, void (*original)(void),
                          const void *context, const char *identity,
                          const void *payload, size_t size,
                          void (*keep)(void *), size_t argc, int result_kind) {
  dev_dynamic_value root = dev_object_root();
  dev_object_lock(&dev_object_global_lock);
  for (dev_object_function *f = dev_object_global_functions; f; f = f->next)
    if (f->context == context &&
        (f->original == original ||
         (identity && f->identity && !strcmp(identity, f->identity)))) {
      dev_dynamic_value v = {f->node, 1};
      dev_object_unlock(&dev_object_global_lock);
      return v;
    }
  dev_dynamic_value v = dev_dynamic_make(8);
  dev_object_function *f = dev_dynamic_alloc(sizeof(*f));
  f->invoke = invoke;
  f->original = original;
  f->context = context;
  f->identity = identity ? dev_dynamic_copy_string(identity) : NULL;
  f->argc = argc;
  f->result_kind = result_kind;
  f->payload = dev_dynamic_alloc(size);
  memcpy(f->payload, payload, size);
  keep(f->payload);
  f->node = (dev_dynamic_node *)v.node;
  f->node->data.opaque = f;
  dev_object_init(&f->meta, root);
  dev_dynamic_value proto = dev_object_new(root);
  dev_object_property prop = dev_object_data_prop("prototype", proto);
  prop.enumerable = false;
  prop.configurable = false;
  dev_object_put_raw(&f->meta, prop);
  prop = dev_object_data_prop("constructor", v);
  prop.enumerable = false;
  dev_object_put_raw(dev_object_meta_of(proto, "<Object>", 0, 0), prop);
  f->next = dev_object_global_functions;
  dev_object_global_functions = f;
  dev_object_unlock(&dev_object_global_lock);
  return v;
}
static dev_dynamic_value dev_object_box_data(dev_dynamic_value v) {
  v.boxed = 1;
  return v;
}
static bool dev_object_same(dev_dynamic_value a, dev_dynamic_value b) {
  if (a.node == b.node)
    return true;
  if (a.boxed || b.boxed)
    return false;
  int at = dev_dynamic_tag(a), bt = dev_dynamic_tag(b);
  if (at != bt || a.scalar_type != b.scalar_type)
    return false;
  if (at == 1)
    return dev_dynamic_node_of(a)->data.boolean ==
           dev_dynamic_node_of(b)->data.boolean;
  if (at == 2)
    return dev_dynamic_node_of(a)->data.integer ==
           dev_dynamic_node_of(b)->data.integer;
  if (at == 3)
    return dev_dynamic_node_of(a)->data.unsigned_integer ==
           dev_dynamic_node_of(b)->data.unsigned_integer;
  if (at == 4) {
    double x = dev_dynamic_node_of(a)->data.number,
           y = dev_dynamic_node_of(b)->data.number;
    return (isnan(x) && isnan(y)) ||
           (x == y && (x != 0 || signbit(x) == signbit(y)));
  }
  if (at == 5)
    return !strcmp(dev_dynamic_node_of(a)->data.string,
                   dev_dynamic_node_of(b)->data.string);
  return false;
}
static dev_dynamic_value dev_object_invoke(dev_dynamic_value value, size_t argc,
                                           const dev_dynamic_value *argv,
                                           const char *p, size_t l, size_t c) {
  if (!value.node || dev_dynamic_node_of(value)->kind != 8)
    dev_dynamic_error("object property is not callable", p, l, c);
  dev_object_function *f = dev_dynamic_node_of(value)->data.opaque;
  if (f->builtin == 1)
    return dev_object_api("object", argc, argv, p, l, c);
  if (f->builtin == 2)
    return dev_dynamic_unboxed(dev_dynamic_string(
        argc && argv[0].node && dev_dynamic_node_of(argv[0])->kind == 8
            ? "[object Function]"
            : "[object Object]"));
  if (f->builtin == 3) {
    if (!argc)
      dev_dynamic_error("valueOf needs a receiver", p, l, c);
    return argv[0];
  }
  if (f->builtin == 100)
    return dev_object_api(f->name, argc, argv, p, l, c);
  return f->invoke(f->payload, argc, argv, p, l, c);
}
static dev_dynamic_value dev_object_method(dev_dynamic_value object,
                                           const char *name, size_t argc,
                                           const dev_dynamic_value *argv,
                                           const char *p, size_t l, size_t c) {
  dev_dynamic_value value = dev_object_get(object, name, p, l, c);
  if (dev_dynamic_tag(value) != 8)
    dev_dynamic_error("object property is not callable", p, l, c);
  dev_object_function *fn = dev_dynamic_node_of(value)->data.opaque;
  if (fn->builtin == 100 || fn->builtin == 1)
    return dev_object_invoke(value, argc, argv, p, l, c);
  dev_dynamic_value *args = dev_dynamic_alloc((argc + 1) * sizeof(*args));
  args[0] = object;
  if (argc)
    memcpy(args + 1, argv, argc * sizeof(*args));
  return dev_object_invoke(value, argc + 1, args, p, l, c);
}
static dev_dynamic_value
dev_object_update_path(dev_dynamic_value root, size_t len,
                       const dev_dynamic_value *path, dev_dynamic_value value,
                       const char *op, const char *p, size_t l, size_t c) {
  if (!len)
    return value;
  const char *key = dev_dynamic_as_string(path[0], p, l, c);
  if (len > 1) {
    dev_dynamic_value child = dev_object_get(root, key, p, l, c);
    dev_dynamic_value next = dev_dynamic_update_inner(child, len - 1, path + 1,
                                                      value, op, p, l, c, 1);
    if (next.node != child.node)
      dev_object_set(root, key, next, p, l, c);
    return root;
  }
  if (strcmp(op, "=")) {
    dev_dynamic_value old = dev_object_get(root, key, p, l, c);
    char binary[3] = {0};
    size_t n = strlen(op);
    if (n < 2 || n > 3)
      dev_dynamic_error("invalid assignment", p, l, c);
    memcpy(binary, op, n - 1);
    value = dev_dynamic_typed(dev_dynamic_binary(binary, old, value, p, l, c),
                              old.scalar_type);
  }
  dev_object_set(root, key, value, p, l, c);
  return root;
}
static size_t dev_object_props(dev_dynamic_value v, dev_object_property **out,
                               const char *p, size_t l, size_t c) {
  if (dev_object_managed(v)) {
    dev_object_meta *m = dev_object_meta_of(v, p, l, c);
    dev_object_lock(&m->lock);
    size_t n = m->len;
    *out = dev_dynamic_alloc(n * sizeof(**out));
    if (n)
      memcpy(*out, m->props, n * sizeof(**out));
    dev_object_unlock(&m->lock);
    return n;
  }
  if (dev_dynamic_tag(v) != 7)
    dev_dynamic_error("expected object", p, l, c);
  const dev_dynamic_node *node = dev_dynamic_node_of(v);
  size_t n = node->data.list.len;
  *out = dev_dynamic_alloc(n * sizeof(**out));
  for (size_t i = 0; i < n; i++) {
    dev_dynamic_value value = node->data.list.values[i];
    if (dev_dynamic_tag(value) > 0 && dev_dynamic_tag(value) < 6)
      value = dev_dynamic_unboxed(value);
    (*out)[i] = dev_object_data_prop(node->data.list.keys[i], value);
  }
  return n;
}
static dev_dynamic_value dev_object_snapshot_inner(dev_dynamic_value v,
                                                   unsigned depth,
                                                   const char *p, size_t l,
                                                   size_t c) {
  if (depth > 128)
    dev_dynamic_error(
        "object serialization nesting exceeds 128; cycles are not serializable",
        p, l, c);
  if (v.node && dev_dynamic_node_of(v)->kind == 8)
    return (dev_dynamic_value){NULL, 1};
  if (v.node && dev_dynamic_node_of(v)->kind == 9) {
    dev_object_property *props;
    size_t n = dev_object_props(v, &props, p, l, c);
    const char **keys = dev_dynamic_alloc(n * sizeof(*keys));
    dev_dynamic_value *values = dev_dynamic_alloc(n * sizeof(*values));
    size_t count = 0;
    for (size_t i = 0; i < n; i++)
      if (props[i].enumerable) {
        dev_dynamic_value value = dev_object_get(v, props[i].key, p, l, c);
        if (value.node && dev_dynamic_node_of(value)->kind == 8)
          continue;
        keys[count] = props[i].key;
        values[count++] = dev_object_snapshot_inner(value, depth + 1, p, l, c);
      }
    return dev_dynamic_object(count, keys, values);
  }
  int tag = dev_dynamic_tag(v);
  if (tag == 6 || tag == 7) {
    const dev_dynamic_node *node = dev_dynamic_node_of(v);
    size_t n = node->data.list.len;
    dev_dynamic_value *values = dev_dynamic_alloc(n * sizeof(*values));
    for (size_t i = 0; i < n; i++)
      values[i] = dev_object_snapshot_inner(node->data.list.values[i],
                                            depth + 1, p, l, c);
    return tag == 6 ? dev_dynamic_array(n, values)
                    : dev_dynamic_object(n, node->data.list.keys, values);
  }
  v.boxed = 1;
  return v;
}
static bool dev_object_contains_reference(dev_dynamic_value v, unsigned depth) {
  if (!v.node)
    return false;
  const dev_dynamic_node *n = dev_dynamic_node_of(v);
  if (n->kind >= 8)
    return true;
  if (depth > 128)
    return true;
  if (n->kind == 6 || n->kind == 7)
    for (size_t i = 0; i < n->data.list.len; i++)
      if (dev_object_contains_reference(n->data.list.values[i], depth + 1))
        return true;
  return false;
}
static dev_dynamic_value dev_object_data_snapshot(dev_dynamic_value v,
                                                  const char *p, size_t l,
                                                  size_t c) {
  if (!dev_object_contains_reference(v, 0)) {
    v.boxed = 1;
    return v;
  }
  return dev_object_snapshot_inner(v, 0, p, l, c);
}
static dev_dynamic_value dev_object_literal(size_t n, const char **keys,
                                            const dev_dynamic_value *values) {
  dev_dynamic_value v = dev_object_new(dev_object_root());
  dev_object_meta *m = dev_object_meta_of(v, "<Object>", 0, 0);
  for (size_t i = 0; i < n; i++)
    dev_object_put_raw(
        m, dev_object_data_prop(dev_dynamic_copy_string(keys[i]), values[i]));
  return v;
}
static bool dev_object_descriptor_field(dev_dynamic_value d, const char *key,
                                        dev_dynamic_value *out, const char *p,
                                        size_t l, size_t c) {
  dev_object_property property;
  if (!dev_object_lookup(d, key, &property, p, l, c))
    return false;
  *out = dev_object_get(d, key, p, l, c);
  return true;
}
static void dev_object_define(dev_dynamic_value object, const char *key,
                              dev_dynamic_value descriptor, const char *p,
                              size_t l, size_t c) {
  dev_dynamic_value value = {0}, get = {0}, set = {0}, writable = {0},
                    enumerable = {0}, configurable = {0};
  bool hv, hg, hs, hw, he, hc;
  if (dev_dynamic_tag(descriptor) == 8) {
    hv = hw = hc = true;
    hg = hs = he = false;
    value = descriptor;
    writable = dev_dynamic_bool(true);
    configurable = dev_dynamic_bool(true);
  } else {
    hv = dev_object_descriptor_field(descriptor, "value", &value, p, l, c);
    hg = dev_object_descriptor_field(descriptor, "get", &get, p, l, c);
    hs = dev_object_descriptor_field(descriptor, "set", &set, p, l, c);
    hw =
        dev_object_descriptor_field(descriptor, "writable", &writable, p, l, c);
    he = dev_object_descriptor_field(descriptor, "enumerable", &enumerable, p,
                                     l, c);
    hc = dev_object_descriptor_field(descriptor, "configurable", &configurable,
                                     p, l, c);
  }
  if ((hg || hs) && (hv || hw))
    dev_dynamic_error("descriptor cannot mix value/writable with get/set", p, l,
                      c);
  if ((hg && get.node && dev_dynamic_tag(get) != 8) ||
      (hs && set.node && dev_dynamic_tag(set) != 8))
    dev_dynamic_error("descriptor getter/setter must be functions or null", p,
                      l, c);
  bool bw = hw ? dev_dynamic_as_bool(writable, p, l, c) : false,
       be = he ? dev_dynamic_as_bool(enumerable, p, l, c) : false,
       bc = hc ? dev_dynamic_as_bool(configurable, p, l, c) : false;
  dev_object_meta *m = dev_object_meta_of(object, p, l, c);
  dev_object_lock(&m->lock);
  bool found = false;
  dev_object_property old = {0};
  for (size_t i = 0; i < m->len; i++)
    if (!strcmp(m->props[i].key, key)) {
      old = m->props[i];
      found = true;
      break;
    }
  if (!found && !m->extensible) {
    dev_object_unlock(&m->lock);
    dev_dynamic_error("object is not extensible", p, l, c);
  }
  dev_object_property next =
      found ? old
            : (dev_object_property){dev_dynamic_copy_string(key),
                                    (dev_dynamic_value){NULL, 1},
                                    {0},
                                    {0},
                                    false,
                                    false,
                                    false,
                                    false};
  if (hg || hs) {
    next.accessor = true;
    next.value = (dev_dynamic_value){0};
    next.writable = false;
  }
  if (hv || hw) {
    next.accessor = false;
    next.get = (dev_dynamic_value){0};
    next.set = (dev_dynamic_value){0};
  }
  if (hv)
    next.value = value;
  if (hg)
    next.get = get;
  if (hs)
    next.set = set;
  if (hw)
    next.writable = bw;
  if (he)
    next.enumerable = be;
  if (hc)
    next.configurable = bc;
  if (found && !old.configurable) {
    bool bad = next.configurable || next.enumerable != old.enumerable ||
               next.accessor != old.accessor;
    if (!old.accessor && !old.writable)
      bad |= next.writable || (hv && !dev_object_same(old.value, next.value));
    if (old.accessor)
      bad |= (hg && !dev_object_same(old.get, next.get)) ||
             (hs && !dev_object_same(old.set, next.set));
    if (bad) {
      dev_object_unlock(&m->lock);
      dev_dynamic_error("cannot redefine non-configurable property", p, l, c);
    }
  }
  dev_object_put_raw(m, next);
  dev_object_unlock(&m->lock);
}
static dev_dynamic_value dev_object_descriptor(dev_object_property property) {
  dev_dynamic_value result = dev_object_new((dev_dynamic_value){0});
  dev_object_meta *m = dev_object_meta_of(result, "<Object>", 0, 0);
  dev_object_put_raw(m, dev_object_data_prop(
                            "enumerable", dev_dynamic_unboxed(dev_dynamic_bool(
                                              property.enumerable))));
  dev_object_put_raw(
      m, dev_object_data_prop(
             "configurable",
             dev_dynamic_unboxed(dev_dynamic_bool(property.configurable))));
  if (property.accessor) {
    dev_object_put_raw(m, dev_object_data_prop("get", property.get));
    dev_object_put_raw(m, dev_object_data_prop("set", property.set));
  } else {
    dev_object_put_raw(m, dev_object_data_prop("value", property.value));
    dev_object_put_raw(m, dev_object_data_prop(
                              "writable", dev_dynamic_unboxed(dev_dynamic_bool(
                                              property.writable))));
  }
  return result;
}
static dev_dynamic_value dev_object_api(const char *name, size_t argc,
                                        const dev_dynamic_value *argv,
                                        const char *p, size_t l, size_t c) {
  if (!strcmp(name, "array")) {
    size_t count = argc;
    bool length = argc == 1 && !argv[0].boxed &&
                  dev_dynamic_tag(argv[0]) >= 2 &&
                  dev_dynamic_tag(argv[0]) <= 4;
    if (length) {
      double n = dev_dynamic_as_float(argv[0], p, l, c);
      if (!isfinite(n) || n < 0 || n > 1000000 || floor(n) != n)
        dev_dynamic_error(
            "Array length must be between 0 and 1000000 and an integer", p, l,
            c);
      count = (size_t)n;
    }
    dev_dynamic_value *items = dev_dynamic_alloc(count * sizeof(*items));
    for (size_t i = 0; i < count; i++)
      items[i] =
          length ? (dev_dynamic_value){NULL, 1} : dev_object_box_data(argv[i]);
    return dev_dynamic_array(count, items);
  }
  size_t arity = SIZE_MAX;
  if (!strcmp(name, "prototype") || !strcmp(name, "constructor"))
    arity = 0;
  else if (!strcmp(name, "defineProperty"))
    arity = 3;
  else if (!strcmp(name, "defineProperties") || !strcmp(name, "delete") ||
           !strcmp(name, "getOwnPropertyDescriptor") ||
           !strcmp(name, "groupBy"))
    arity = 2;
  else if (strcmp(name, "object") && strcmp(name, "create") &&
           strcmp(name, "assign") && strcmp(name, "construct"))
    arity = 1;
  if (arity != SIZE_MAX && arity != argc)
    dev_dynamic_error("wrong Object API argument count", p, l, c);
  if (!strcmp(name, "prototype"))
    return dev_object_root();
  if (!strcmp(name, "constructor"))
    return dev_object_get(dev_object_root(), "constructor", p, l, c);
  if (!strcmp(name, "static"))
    return dev_object_get(dev_object_api("constructor", 0, NULL, p, l, c),
                          dev_dynamic_as_string(argv[0], p, l, c), p, l, c);
  if (!strcmp(name, "object")) {
    if (argc > 1)
      dev_dynamic_error("Object accepts zero or one data object", p, l, c);
    if (argc && dev_object_managed(argv[0]))
      return argv[0];
    dev_dynamic_value object = dev_object_new(dev_object_root());
    if (argc) {
      dev_object_property *props;
      size_t n = dev_object_props(argv[0], &props, p, l, c);
      for (size_t i = 0; i < n; i++)
        if (props[i].enumerable)
          dev_object_set(object, props[i].key,
                         dev_object_get(argv[0], props[i].key, p, l, c), p, l,
                         c);
    }
    return object;
  }
  if (!strcmp(name, "create")) {
    if (argc > 2)
      dev_dynamic_error(
          "Object.create expects prototype and optional descriptors", p, l, c);
    dev_dynamic_value parent = argc ? argv[0] : dev_object_root();
    if (parent.node) {
      dev_object_property *props;
      (void)dev_object_props(parent, &props, p, l, c);
    }
    dev_dynamic_value object = dev_object_new(parent);
    if (argc == 2) {
      dev_object_property *props;
      size_t n = dev_object_props(argv[1], &props, p, l, c);
      for (size_t i = 0; i < n; i++)
        dev_object_define(object, props[i].key,
                          dev_object_get(argv[1], props[i].key, p, l, c), p, l,
                          c);
    }
    return object;
  }
  if (!strcmp(name, "construct")) {
    if (!argc)
      dev_dynamic_error("Object.construct needs a constructor", p, l, c);
    dev_dynamic_value proto = dev_object_get(argv[0], "prototype", p, l, c),
                      object = dev_object_new(proto);
    dev_dynamic_value *args = dev_dynamic_alloc(argc * sizeof(*args));
    args[0] = object;
    if (argc > 1)
      memcpy(args + 1, argv + 1, (argc - 1) * sizeof(*args));
    dev_dynamic_value result = dev_object_invoke(argv[0], argc, args, p, l, c);
    return result.node && dev_dynamic_node_of(result)->kind == 9 ? result
                                                                 : object;
  }
  if (!strcmp(name, "assign")) {
    if (!argc)
      dev_dynamic_error("Object.assign needs a target", p, l, c);
    (void)dev_object_meta_of(argv[0], p, l, c);
    for (size_t j = 1; j < argc; j++) {
      dev_object_property *props;
      size_t n = dev_object_props(argv[j], &props, p, l, c);
      for (size_t i = 0; i < n; i++)
        if (props[i].enumerable)
          dev_object_set(argv[0], props[i].key,
                         dev_object_get(argv[j], props[i].key, p, l, c), p, l,
                         c);
    }
    return argv[0];
  }
  if (!strcmp(name, "defineProperty")) {
    dev_object_define(argv[0], dev_dynamic_as_string(argv[1], p, l, c), argv[2],
                      p, l, c);
    return argv[0];
  }
  if (!strcmp(name, "defineProperties")) {
    dev_object_property *props;
    size_t n = dev_object_props(argv[1], &props, p, l, c);
    for (size_t i = 0; i < n; i++)
      dev_object_define(argv[0], props[i].key,
                        dev_object_get(argv[1], props[i].key, p, l, c), p, l,
                        c);
    return argv[0];
  }
  if (!strcmp(name, "delete")) {
    dev_object_meta *m = dev_object_meta_of(argv[0], p, l, c);
    const char *key = dev_dynamic_as_string(argv[1], p, l, c);
    dev_object_lock(&m->lock);
    size_t i = 0;
    while (i < m->len && strcmp(m->props[i].key, key))
      ++i;
    if (i < m->len && !m->props[i].configurable) {
      dev_object_unlock(&m->lock);
      return dev_dynamic_unboxed(dev_dynamic_bool(false));
    }
    if (i < m->len) {
      dev_object_property *next =
          dev_dynamic_alloc((m->len - 1) * sizeof(*next));
      if (i)
        memcpy(next, m->props, i * sizeof(*next));
      if (m->len - i - 1)
        memcpy(next + i, m->props + i + 1, (m->len - i - 1) * sizeof(*next));
      m->props = next;
      --m->len;
    }
    dev_object_unlock(&m->lock);
    return dev_dynamic_unboxed(dev_dynamic_bool(true));
  }
  if (!strcmp(name, "freeze") || !strcmp(name, "seal") ||
      !strcmp(name, "preventExtensions")) {
    dev_object_meta *m = dev_object_meta_of(argv[0], p, l, c);
    dev_object_lock(&m->lock);
    m->extensible = false;
    if (strcmp(name, "preventExtensions")) {
      dev_object_property *next = dev_dynamic_alloc(m->len * sizeof(*next));
      if (m->len)
        memcpy(next, m->props, m->len * sizeof(*next));
      for (size_t i = 0; i < m->len; i++) {
        next[i].configurable = false;
        if (!strcmp(name, "freeze") && !next[i].accessor)
          next[i].writable = false;
      }
      m->props = next;
    }
    dev_object_unlock(&m->lock);
    return argv[0];
  }
  if (!strcmp(name, "isExtensible") || !strcmp(name, "isSealed") ||
      !strcmp(name, "isFrozen")) {
    dev_object_meta *m = dev_object_meta_of(argv[0], p, l, c);
    dev_object_lock(&m->lock);
    bool result =
        !strcmp(name, "isExtensible") ? m->extensible : !m->extensible;
    if (strcmp(name, "isExtensible"))
      for (size_t i = 0; i < m->len; i++)
        if (m->props[i].configurable ||
            (!strcmp(name, "isFrozen") && !m->props[i].accessor &&
             m->props[i].writable))
          result = false;
    dev_object_unlock(&m->lock);
    return dev_dynamic_unboxed(dev_dynamic_bool(result));
  }
  if (!strcmp(name, "keys") || !strcmp(name, "getOwnPropertyNames") ||
      !strcmp(name, "values") || !strcmp(name, "entries")) {
    dev_object_property *props;
    size_t n = dev_object_props(argv[0], &props, p, l, c);
    dev_dynamic_value *out = dev_dynamic_alloc(n * sizeof(*out));
    size_t count = 0;
    for (size_t i = 0; i < n; i++) {
      if (strcmp(name, "getOwnPropertyNames") && !props[i].enumerable)
        continue;
      dev_dynamic_value key = dev_dynamic_string(props[i].key);
      if (!strcmp(name, "keys") || !strcmp(name, "getOwnPropertyNames"))
        out[count++] = key;
      else {
        dev_dynamic_value value =
            dev_object_box_data(dev_object_get(argv[0], props[i].key, p, l, c));
        if (!strcmp(name, "values"))
          out[count++] = value;
        else {
          dev_dynamic_value pair[] = {key, value};
          out[count++] = dev_dynamic_array(2, pair);
        }
      }
    }
    return dev_dynamic_array(count, out);
  }
  if (!strcmp(name, "getOwnPropertyDescriptor")) {
    dev_object_property property;
    if (!dev_object_own_prop(argv[0], dev_dynamic_as_string(argv[1], p, l, c),
                             &property, p, l, c))
      return (dev_dynamic_value){NULL, 1};
    return dev_object_descriptor(property);
  }
  if (!strcmp(name, "getOwnPropertyDescriptors")) {
    dev_dynamic_value object = dev_object_new((dev_dynamic_value){0});
    dev_object_property *props;
    size_t n = dev_object_props(argv[0], &props, p, l, c);
    for (size_t i = 0; i < n; i++)
      dev_object_set(object, props[i].key, dev_object_descriptor(props[i]), p,
                     l, c);
    return object;
  }
  if (!strcmp(name, "fromEntries")) {
    if (dev_dynamic_tag(argv[0]) != 6)
      dev_dynamic_error("fromEntries requires an iterable of pairs", p, l, c);
    dev_dynamic_value object = dev_object_new(dev_object_root());
    const dev_dynamic_node *list = dev_dynamic_node_of(argv[0]);
    for (size_t i = 0; i < list->data.list.len; i++) {
      dev_dynamic_value pair = list->data.list.values[i];
      if (dev_dynamic_tag(pair) != 6 ||
          dev_dynamic_node_of(pair)->data.list.len != 2)
        dev_dynamic_error("fromEntries requires key/value pairs", p, l, c);
      const dev_dynamic_node *row = dev_dynamic_node_of(pair);
      dev_dynamic_value value = row->data.list.values[1];
      if (dev_dynamic_tag(value) > 0 && dev_dynamic_tag(value) < 6)
        value = dev_dynamic_unboxed(value);
      dev_object_set(object,
                     dev_dynamic_as_string(row->data.list.values[0], p, l, c),
                     value, p, l, c);
    }
    return object;
  }
  if (!strcmp(name, "groupBy")) {
    if (dev_dynamic_tag(argv[0]) != 6 || dev_dynamic_tag(argv[1]) != 8)
      dev_dynamic_error("groupBy requires a collection and a Dev function", p,
                        l, c);
    dev_dynamic_value object = dev_object_new((dev_dynamic_value){0});
    const dev_dynamic_node *list = dev_dynamic_node_of(argv[0]);
    dev_object_function *function = dev_dynamic_node_of(argv[1])->data.opaque;
    if (function->builtin || function->result_kind != 5 ||
        (function->argc != 1 && function->argc != 2))
      dev_dynamic_error("groupBy callback must return str and accept item plus "
                        "optional index",
                        p, l, c);
    for (size_t i = 0; i < list->data.list.len; i++) {
      dev_dynamic_value args[] = {
          list->data.list.values[i],
          dev_dynamic_typed(dev_dynamic_int((int64_t)i), 2)};
      dev_dynamic_value key =
          dev_object_invoke(argv[1], function->argc, args, p, l, c);
      const char *text = dev_dynamic_as_string(key, p, l, c);
      dev_object_property property;
      dev_dynamic_value group =
          dev_object_own_prop(object, text, &property, p, l, c)
              ? property.value
              : dev_dynamic_array(0, NULL);
      group = dev_dynamic_push(
          group, dev_object_box_data(list->data.list.values[i]), p, l, c);
      dev_object_set(object, text, group, p, l, c);
    }
    return object;
  }
  if (!strcmp(name, "toString"))
    return dev_dynamic_unboxed(dev_dynamic_string(
        argv[0].node && dev_dynamic_node_of(argv[0])->kind == 8
            ? "[object Function]"
            : "[object Object]"));
  if (!strcmp(name, "valueOf"))
    return argv[0];
  dev_dynamic_error("unknown Object API", p, l, c);
  return (dev_dynamic_value){0};
}
