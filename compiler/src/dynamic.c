/* Hosted native immutable data values. Each update copies its ancestor path.
   Allocation roots live until process exit, like native interpolated strings.
   Nodes never change after construction and may safely cross module/thread edges. */
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <math.h>
#include <limits.h>
#include <stdatomic.h>

typedef struct dev_dynamic_node {
    int kind; /* 1 bool, 2 i64, 3 u64, 4 f64, 5 string, 6 array, 7 object */
    union { bool boolean; int64_t integer; uint64_t unsigned_integer; double number;
        const char *string;
        struct { size_t len; const char **keys; dev_dynamic_value *values; } list;
    } data;
} dev_dynamic_node;
typedef struct dev_dynamic_allocation { struct dev_dynamic_allocation *next; void *data; } dev_dynamic_allocation;
static dev_dynamic_allocation *dev_dynamic_allocations;
static atomic_flag dev_dynamic_allocation_lock = ATOMIC_FLAG_INIT;
static inline void dev_dynamic_error(const char *message, const char *file, size_t line, size_t col) {
    fprintf(stderr,"%s:%zu:%zu: %s\n",file,line,col,message); exit(1);
}
static inline void *dev_dynamic_alloc(size_t size) {
    void *data=calloc(1,size?size:1);
    dev_dynamic_allocation *root=(dev_dynamic_allocation*)malloc(sizeof(*root));
    if(!data||!root) { fputs("native DON allocation failed\n",stderr); exit(1); }
    root->data=data;
    while(atomic_flag_test_and_set_explicit(&dev_dynamic_allocation_lock,memory_order_acquire)) {}
    root->next=dev_dynamic_allocations;dev_dynamic_allocations=root;
    atomic_flag_clear_explicit(&dev_dynamic_allocation_lock,memory_order_release);
    return data;
}
static inline const char *dev_dynamic_copy_string(const char *s) {
    if(!s) { fputs("null native string in DON value\n",stderr);exit(1); }
    size_t n=strlen(s);if(n>8*1024*1024) { fputs("DON string exceeds 8 MiB\n",stderr);exit(1); }
    char *p=(char*)dev_dynamic_alloc(n+1);memcpy(p,s,n+1);return p;
}
static inline dev_dynamic_value dev_dynamic_make(int kind) {
    dev_dynamic_node *n=(dev_dynamic_node*)dev_dynamic_alloc(sizeof(*n));n->kind=kind;
    return (dev_dynamic_value){n,1};
}
static inline const dev_dynamic_node *dev_dynamic_node_of(dev_dynamic_value v) {return (const dev_dynamic_node*)v.node;}
static inline int dev_dynamic_tag(dev_dynamic_value v) {return v.node?dev_dynamic_node_of(v)->kind:0;}
static inline dev_dynamic_value dev_dynamic_unboxed(dev_dynamic_value v) {v.boxed=0;int tag=dev_dynamic_tag(v);if(tag==3&&dev_dynamic_node_of(v)->data.unsigned_integer<=INT64_MAX)tag=2;v.scalar_type=(unsigned char)tag;return v;}

static inline dev_dynamic_value dev_dynamic_boxed(dev_dynamic_value v) {v.boxed=1;return v;}
static inline dev_dynamic_value dev_dynamic_require_boxed(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    if(!v.boxed)dev_dynamic_error("expected json.Value, found scalar",file,line,col);return v;
}
static inline dev_dynamic_value dev_dynamic_require(dev_dynamic_value v,int expected,const char *file,size_t line,size_t col) {
    int tag=dev_dynamic_tag(v);
    if(tag==3&&dev_dynamic_node_of(v)->data.unsigned_integer<=INT64_MAX)tag=2;
    if(v.boxed||v.scalar_type!=expected||!expected)dev_dynamic_error("DON scalar type mismatch; use an explicit cast",file,line,col);
    return v;
}
static inline dev_dynamic_value dev_dynamic_bool(bool v) {dev_dynamic_value x=dev_dynamic_make(1);((dev_dynamic_node*)x.node)->data.boolean=v;return x;}
static inline dev_dynamic_value dev_dynamic_int(int64_t v) {dev_dynamic_value x=dev_dynamic_make(2);((dev_dynamic_node*)x.node)->data.integer=v;return x;}
static inline dev_dynamic_value dev_dynamic_uint(uint64_t v) {dev_dynamic_value x=dev_dynamic_make(3);((dev_dynamic_node*)x.node)->data.unsigned_integer=v;return x;}
static inline dev_dynamic_value dev_dynamic_float(double v) {
    if(!isfinite(v)) {fputs("DON numbers must be finite\n",stderr);exit(1);}
    dev_dynamic_value x=dev_dynamic_make(4);((dev_dynamic_node*)x.node)->data.number=v;return x;
}
static inline dev_dynamic_value dev_dynamic_string(const char *v) {dev_dynamic_value x=dev_dynamic_make(5);((dev_dynamic_node*)x.node)->data.string=dev_dynamic_copy_string(v);return x;}
static inline dev_dynamic_value dev_dynamic_array(size_t len,const dev_dynamic_value *values) {
    if(len>SIZE_MAX/sizeof(dev_dynamic_value)){fputs("DON array too large\n",stderr);exit(1);}
    dev_dynamic_value v=dev_dynamic_make(6);dev_dynamic_node *n=(dev_dynamic_node*)v.node;
    n->data.list.len=len;n->data.list.values=(dev_dynamic_value*)dev_dynamic_alloc(len*sizeof(*values));
    if(len)memcpy(n->data.list.values,values,len*sizeof(*values));return v;
}
static inline dev_dynamic_value dev_dynamic_object(size_t len,const char **keys,const dev_dynamic_value *values) {
    dev_dynamic_value v=dev_dynamic_array(len,values);dev_dynamic_node *n=(dev_dynamic_node*)v.node;n->kind=7;
    n->data.list.keys=(const char**)dev_dynamic_alloc(len*sizeof(*keys));
    for(size_t i=0;i<len;i++)n->data.list.keys[i]=dev_dynamic_copy_string(keys[i]);
    /* serde_json's default map is sorted; preserve stable serialization and keys. */
    for(size_t i=1;i<len;i++) {size_t j=i;while(j && strcmp(n->data.list.keys[j-1],n->data.list.keys[j])>0) {
        const char *key=n->data.list.keys[j-1];n->data.list.keys[j-1]=n->data.list.keys[j];n->data.list.keys[j]=key;
        dev_dynamic_value item=n->data.list.values[j-1];n->data.list.values[j-1]=n->data.list.values[j];n->data.list.values[j]=item;j--;
    }}return v;
}
static inline const char *dev_dynamic_as_string(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    if(dev_dynamic_tag(v)!=5)dev_dynamic_error("expected DON string",file,line,col);return dev_dynamic_node_of(v)->data.string;
}
static inline bool dev_dynamic_as_bool(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    if(dev_dynamic_tag(v)!=1)dev_dynamic_error("expected DON bool",file,line,col);return dev_dynamic_node_of(v)->data.boolean;
}
static inline int64_t dev_dynamic_as_int(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    const dev_dynamic_node *n=dev_dynamic_node_of(v);int tag=dev_dynamic_tag(v);
    if(tag==2)return n->data.integer;
    if(tag==3&&n->data.unsigned_integer<=INT64_MAX)return (int64_t)n->data.unsigned_integer;
    dev_dynamic_error("expected DON i64 integer",file,line,col);return 0;
}
static inline uint64_t dev_dynamic_as_uint(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    const dev_dynamic_node *n=dev_dynamic_node_of(v);int tag=dev_dynamic_tag(v);
    if(tag==3)return n->data.unsigned_integer;
    if(tag==2&&n->data.integer>=0)return (uint64_t)n->data.integer;
    dev_dynamic_error("expected DON unsigned integer",file,line,col);return 0;
}
static inline uint64_t dev_dynamic_checked_integer(dev_dynamic_value v,unsigned bits,bool signed_value,const char *file,size_t line,size_t col) {
    if(signed_value) {
        int64_t n=dev_dynamic_as_int(v,file,line,col);
        if(bits<64 && (n<-(INT64_C(1)<<(bits-1))||n>((INT64_C(1)<<(bits-1))-1)))dev_dynamic_error("DON integer out of range",file,line,col);
        return (uint64_t)n;
    }
    uint64_t n=dev_dynamic_as_uint(v,file,line,col);
    if(bits<64 && n>((UINT64_C(1)<<bits)-1))dev_dynamic_error("DON integer out of range",file,line,col);return n;
}
static inline double dev_dynamic_as_float(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    const dev_dynamic_node *n=dev_dynamic_node_of(v);switch(dev_dynamic_tag(v)) {
        case 2:return (double)n->data.integer;case 3:return (double)n->data.unsigned_integer;case 4:return n->data.number;
        default:dev_dynamic_error("expected DON number",file,line,col);return 0;
    }
}
static inline dev_dynamic_value dev_dynamic_typed(dev_dynamic_value v,unsigned type) {
    if(type>10) {unsigned bits=type<128?type-10:type-138;bool signed_value=type>=128;
        if(bits>0&&bits<64) {const dev_dynamic_node *n=dev_dynamic_node_of(v);uint64_t raw=n->kind==2?(uint64_t)n->data.integer:n->data.unsigned_integer;
            uint64_t mask=(UINT64_C(1)<<bits)-1;raw&=mask;
            if(signed_value) {if(raw&(UINT64_C(1)<<(bits-1)))raw|=~mask;v=dev_dynamic_int((int64_t)raw);}else v=dev_dynamic_uint(raw);
        }
    }v.boxed=0;v.scalar_type=(unsigned char)type;return v;
}
static inline uint64_t dev_dynamic_cast_integer(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    if(v.boxed)dev_dynamic_error("cannot cast boxed json.Value to scalar",file,line,col);
    const dev_dynamic_node *n=dev_dynamic_node_of(v);switch(dev_dynamic_tag(v)){case 1:return n->data.boolean;case 2:return (uint64_t)n->data.integer;case 3:return n->data.unsigned_integer;default:dev_dynamic_error("numeric cast requires number or bool",file,line,col);return 0;}
}
static inline double dev_dynamic_cast_float(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    if(v.boxed)dev_dynamic_error("cannot cast boxed json.Value to scalar",file,line,col);
    if(dev_dynamic_tag(v)==1)return dev_dynamic_node_of(v)->data.boolean?1:0;return dev_dynamic_as_float(v,file,line,col);
}
static inline bool dev_dynamic_cast_bool(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    if(dev_dynamic_tag(v)==4)return dev_dynamic_cast_float(v,file,line,col)!=0;
    return dev_dynamic_cast_integer(v,file,line,col)!=0;
}
static inline const char *dev_dynamic_kind(dev_dynamic_value v) {
    const char *names[]={"null","bool","number","number","number","string","array","object"};return names[dev_dynamic_tag(v)];
}
static inline size_t dev_dynamic_len(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    int tag=dev_dynamic_tag(v);if(tag!=6&&tag!=7)dev_dynamic_error("DON len requires object or array",file,line,col);return dev_dynamic_node_of(v)->data.list.len;
}
static inline const char **dev_dynamic_keys(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    if(dev_dynamic_tag(v)!=7)dev_dynamic_error("DON keys requires object",file,line,col);return dev_dynamic_node_of(v)->data.list.keys;
}
static inline size_t dev_dynamic_find(dev_dynamic_value v,dev_dynamic_value key,const char *file,size_t line,size_t col) {
    if(key.boxed)dev_dynamic_error("DON key/index requires a scalar",file,line,col);
    int tag=dev_dynamic_tag(v);const dev_dynamic_node *n=dev_dynamic_node_of(v);
    if(tag==7) { const char *s=dev_dynamic_as_string(key,file,line,col);
        for(size_t i=0;i<n->data.list.len;i++)if(strcmp(n->data.list.keys[i],s)==0)return i;return SIZE_MAX;
    }
    if(tag==6) {uint64_t i=dev_dynamic_as_uint(key,file,line,col);if(i>=n->data.list.len)dev_dynamic_error("DON index out of bounds",file,line,col);return (size_t)i;}
    dev_dynamic_error("DON field/index access requires object or array",file,line,col);return SIZE_MAX;
}
static inline dev_dynamic_value dev_dynamic_get(dev_dynamic_value v,dev_dynamic_value key,const char *file,size_t line,size_t col) {
    size_t i=dev_dynamic_find(v,key,file,line,col);if(i==SIZE_MAX)dev_dynamic_error("missing DON object key",file,line,col);dev_dynamic_value result=dev_dynamic_node_of(v)->data.list.values[i];result.boxed=1;return result;
}
static inline dev_dynamic_value dev_dynamic_read(dev_dynamic_value v,dev_dynamic_value key,const char *file,size_t line,size_t col) {
    if(!v.boxed&&dev_dynamic_tag(v)==5) {
        if(key.boxed)dev_dynamic_error("str index requires an integer",file,line,col);
        uint64_t index=dev_dynamic_as_uint(key,file,line,col);const char *s=dev_dynamic_node_of(v)->data.string;
        if(index>=strlen(s))dev_dynamic_error("str index out of bounds",file,line,col);
        return dev_dynamic_typed(dev_dynamic_uint((unsigned char)s[index]),18);
    }
    dev_dynamic_value result=dev_dynamic_get(v,key,file,line,col);int tag=dev_dynamic_tag(result);
    result.boxed=(tag==0||tag>=6);return result.boxed?result:dev_dynamic_unboxed(result);
}
/* Lookup-only keys borrow their call argument, avoiding arena growth on reads. */
static inline dev_dynamic_value dev_dynamic_read_field(dev_dynamic_value v,const char *name,const char *file,size_t line,size_t col) {
    dev_dynamic_node key={0};key.kind=5;key.data.string=name;
    return dev_dynamic_read(v,(dev_dynamic_value){&key,0,5},file,line,col);
}
static inline dev_dynamic_value dev_dynamic_read_path(dev_dynamic_value v,size_t len,const dev_dynamic_value *path,const char *file,size_t line,size_t col) {
    for(size_t i=0;i<len;i++)v=dev_dynamic_read(v,path[i],file,line,col);return v;
}
static inline dev_dynamic_value dev_dynamic_literal(dev_dynamic_value literal,dev_dynamic_value context,const char *file,size_t line,size_t col) {
    if(context.boxed)dev_dynamic_error("numeric literal requires scalar context",file,line,col);
    int tag=dev_dynamic_tag(context),rhs=dev_dynamic_tag(literal);
    if(tag==4&&(rhs==2||rhs==3))return dev_dynamic_unboxed(dev_dynamic_float(dev_dynamic_as_float(literal,file,line,col)));
    if(context.scalar_type==3&&rhs==2)return dev_dynamic_typed(dev_dynamic_uint(dev_dynamic_as_uint(literal,file,line,col)),3);
    if(context.scalar_type>10) {
        unsigned type=context.scalar_type,bits=type<128?type-10:type-138;bool signed_value=type>=128;
        uint64_t number=dev_dynamic_checked_integer(literal,bits,signed_value,file,line,col);
        return dev_dynamic_typed(signed_value?dev_dynamic_int((int64_t)number):dev_dynamic_uint(number),type);
    }
    return literal;
}
static inline bool dev_dynamic_has(dev_dynamic_value v,dev_dynamic_value key,const char *file,size_t line,size_t col) {
    if(dev_dynamic_tag(v)!=7)dev_dynamic_error("DON has requires object",file,line,col);return dev_dynamic_find(v,key,file,line,col)!=SIZE_MAX;
}
static inline dev_dynamic_value dev_dynamic_binary(const char *op,dev_dynamic_value a,dev_dynamic_value b,const char *file,size_t line,size_t col) {
    if(a.boxed||b.boxed)dev_dynamic_error("operator requires Dev scalar operands",file,line,col);
    if(a.scalar_type!=b.scalar_type)dev_dynamic_error("numeric types do not implicitly mix",file,line,col);
    int at=dev_dynamic_tag(a),bt=dev_dynamic_tag(b);const dev_dynamic_node *an=dev_dynamic_node_of(a),*bn=dev_dynamic_node_of(b);
    if(!strcmp(op,"<<")||!strcmp(op,">>")) {
        unsigned type=a.scalar_type,bits=type>10?(type<128?type-10:type-138):64;
        uint64_t shift=dev_dynamic_as_uint(b,file,line,col);
        if(shift>=bits)dev_dynamic_error("invalid shift count",file,line,col);
    }
    bool comparison=!strcmp(op,"==")||!strcmp(op,"!=")||!strcmp(op,"<")||!strcmp(op,"<=")||!strcmp(op,">")||!strcmp(op,">=");
    if(at==5&&bt==5) {
        const char *x=an->data.string,*y=bn->data.string;
        if(!strcmp(op,"+")) {size_t nx=strlen(x),ny=strlen(y);if(nx+ny>8*1024*1024)dev_dynamic_error("DON string exceeds 8 MiB",file,line,col);char *s=(char*)dev_dynamic_alloc(nx+ny+1);memcpy(s,x,nx);memcpy(s+nx,y,ny+1);return dev_dynamic_string(s);}
        int c=strcmp(x,y);if(!strcmp(op,"=="))return dev_dynamic_bool(c==0);if(!strcmp(op,"!="))return dev_dynamic_bool(c!=0);
    }
    if(at==1&&bt==1) {if(!strcmp(op,"=="))return dev_dynamic_bool(an->data.boolean==bn->data.boolean);if(!strcmp(op,"!="))return dev_dynamic_bool(an->data.boolean!=bn->data.boolean);}
    if(at<2||at>4||bt<2||bt>4)dev_dynamic_error("invalid DON operands",file,line,col);
    if(at==4||bt==4) {
        if(at!=4||bt!=4)dev_dynamic_error("numeric types do not implicitly mix",file,line,col);
        double x=an->data.number,y=bn->data.number;
        if(!strcmp(op,"=="))return dev_dynamic_bool(x==y);if(!strcmp(op,"!="))return dev_dynamic_bool(x!=y);
        if(!strcmp(op,"<"))return dev_dynamic_bool(x<y);if(!strcmp(op,"<="))return dev_dynamic_bool(x<=y);if(!strcmp(op,">"))return dev_dynamic_bool(x>y);if(!strcmp(op,">="))return dev_dynamic_bool(x>=y);
        if(!strcmp(op,"+"))return dev_dynamic_float(x+y);if(!strcmp(op,"-"))return dev_dynamic_float(x-y);if(!strcmp(op,"*"))return dev_dynamic_float(x*y);
        if(!strcmp(op,"/")) {if(y==0)dev_dynamic_error("division by zero",file,line,col);return dev_dynamic_float(x/y);}
        dev_dynamic_error("invalid DON floating operator",file,line,col);
    }
    /* JSON positive integers unpack as i64 unless they exceed INT64_MAX. */
    bool unsigned_value=a.scalar_type==3;
    if(unsigned_value) {
        if(at!=3||bt!=3)dev_dynamic_error("numeric types do not implicitly mix",file,line,col);
        uint64_t x=an->data.unsigned_integer,y=bn->data.unsigned_integer;
        if(!strcmp(op,"=="))return dev_dynamic_bool(x==y);if(!strcmp(op,"!="))return dev_dynamic_bool(x!=y);if(!strcmp(op,"<"))return dev_dynamic_bool(x<y);if(!strcmp(op,"<="))return dev_dynamic_bool(x<=y);if(!strcmp(op,">"))return dev_dynamic_bool(x>y);if(!strcmp(op,">="))return dev_dynamic_bool(x>=y);
        if(!strcmp(op,"+"))return dev_dynamic_uint(x+y);if(!strcmp(op,"-"))return dev_dynamic_uint(x-y);if(!strcmp(op,"*"))return dev_dynamic_uint(x*y);
        if(!strcmp(op,"/")||!strcmp(op,"%")){if(!y)dev_dynamic_error("division by zero",file,line,col);return dev_dynamic_uint(*op=='/'?x/y:x%y);}
        if(!strcmp(op,"&"))return dev_dynamic_uint(x&y);if(!strcmp(op,"|"))return dev_dynamic_uint(x|y);if(!strcmp(op,"^"))return dev_dynamic_uint(x^y);
        if(!strcmp(op,"<<")||!strcmp(op,">>")){if(y>=64)dev_dynamic_error("invalid shift count",file,line,col);return dev_dynamic_uint(*op=='<'?x<<y:x>>y);}
    } else {
        int64_t x=dev_dynamic_as_int(a,file,line,col),y=dev_dynamic_as_int(b,file,line,col);
        if(comparison) {if(!strcmp(op,"=="))return dev_dynamic_bool(x==y);if(!strcmp(op,"!="))return dev_dynamic_bool(x!=y);if(!strcmp(op,"<"))return dev_dynamic_bool(x<y);if(!strcmp(op,"<="))return dev_dynamic_bool(x<=y);if(!strcmp(op,">"))return dev_dynamic_bool(x>y);return dev_dynamic_bool(x>=y);}
        if(!strcmp(op,"+"))return dev_dynamic_int((int64_t)((uint64_t)x+(uint64_t)y));if(!strcmp(op,"-"))return dev_dynamic_int((int64_t)((uint64_t)x-(uint64_t)y));if(!strcmp(op,"*"))return dev_dynamic_int((int64_t)((uint64_t)x*(uint64_t)y));
        if(!strcmp(op,"/")||!strcmp(op,"%")){if(!y)dev_dynamic_error("division by zero",file,line,col);if(x==INT64_MIN&&y==-1)dev_dynamic_error("integer division overflow",file,line,col);return dev_dynamic_int(*op=='/'?x/y:x%y);}
        if(!strcmp(op,"&"))return dev_dynamic_int(x&y);if(!strcmp(op,"|"))return dev_dynamic_int(x|y);if(!strcmp(op,"^"))return dev_dynamic_int(x^y);
        if(!strcmp(op,"<<")||!strcmp(op,">>")){if(y<0||y>=64)dev_dynamic_error("invalid shift count",file,line,col);return dev_dynamic_int(*op=='<'?(int64_t)((uint64_t)x<<y):x>>y);}
    }
    dev_dynamic_error("invalid DON operator",file,line,col);return (dev_dynamic_value){0};
}
static inline dev_dynamic_value dev_dynamic_unary(const char *op,dev_dynamic_value v,const char *file,size_t line,size_t col) {
    if(v.boxed)dev_dynamic_error("operator requires a Dev scalar",file,line,col);
    if(!strcmp(op,"!"))return dev_dynamic_bool(!dev_dynamic_as_bool(v,file,line,col));
    if(!strcmp(op,"+")){int tag=dev_dynamic_tag(v);if(tag<2||tag>4)dev_dynamic_error("expected DON number",file,line,col);return v;}
    if(!strcmp(op,"-")&&dev_dynamic_tag(v)==4)return dev_dynamic_float(-dev_dynamic_node_of(v)->data.number);
    if(v.scalar_type==3){uint64_t n=dev_dynamic_as_uint(v,file,line,col);return dev_dynamic_uint(*op=='~'?~n:UINT64_C(0)-n);}
    int64_t n=dev_dynamic_as_int(v,file,line,col);return dev_dynamic_int(*op=='~'?~n:(int64_t)(UINT64_C(0)-(uint64_t)n));
}
static inline dev_dynamic_value dev_dynamic_assign(dev_dynamic_value old,dev_dynamic_value replacement,const char *op,const char *file,size_t line,size_t col) {
    if(!strcmp(op,"=")) {
        int a=dev_dynamic_tag(old),b=dev_dynamic_tag(replacement);
        if(a==3&&dev_dynamic_node_of(old)->data.unsigned_integer<=INT64_MAX)a=2;
        if(b==3&&dev_dynamic_node_of(replacement)->data.unsigned_integer<=INT64_MAX)b=2;
        if(old.boxed!=replacement.boxed||(!old.boxed&&(a!=b||old.scalar_type!=replacement.scalar_type)))dev_dynamic_error("assignment type mismatch",file,line,col);
        return replacement;
    }
    if(old.boxed)dev_dynamic_error("compound assignment requires a scalar",file,line,col);
    char binary[3]={0};size_t n=strlen(op);if(n<2||n>3)dev_dynamic_error("invalid assignment",file,line,col);memcpy(binary,op,n-1);
    return dev_dynamic_typed(dev_dynamic_binary(binary,old,replacement,file,line,col),old.scalar_type);
}
static inline dev_dynamic_value dev_dynamic_update_inner(dev_dynamic_value root,size_t len,const dev_dynamic_value *path,dev_dynamic_value replacement,const char *op,const char *file,size_t line,size_t col,unsigned depth) {
    if(depth>128)dev_dynamic_error("DON nesting limit exceeded",file,line,col);
    if(!len) {if(!strcmp(op,"="))return replacement;char binary[3]={0};size_t n=strlen(op);if(n<2||n>3)dev_dynamic_error("invalid assignment",file,line,col);memcpy(binary,op,n-1);int tag=dev_dynamic_tag(root);if(tag>0&&tag<6)root=dev_dynamic_unboxed(root);return dev_dynamic_binary(binary,root,replacement,file,line,col);}
    size_t i=dev_dynamic_find(root,*path,file,line,col);const dev_dynamic_node *old=dev_dynamic_node_of(root);
    bool insert=i==SIZE_MAX;
    if(insert&&(len!=1||strcmp(op,"=")))dev_dynamic_error("missing DON object key",file,line,col);
    size_t n=old->data.list.len;
    dev_dynamic_value next=dev_dynamic_update_inner(insert?(dev_dynamic_value){0}:old->data.list.values[i],len-1,path+1,replacement,op,file,line,col,depth+1);
    dev_dynamic_value *copied=(dev_dynamic_value*)dev_dynamic_alloc((n+insert)*sizeof(*copied));
    if(n)memcpy(copied,old->data.list.values,n*sizeof(*copied));
    dev_dynamic_value result=dev_dynamic_array(n+insert,copied);
    /* For insertion construct a full array without reading beyond old storage. */
    if(insert) {dev_dynamic_node *r=(dev_dynamic_node*)result.node;r->data.list.values[n]=next;} else {((dev_dynamic_node*)result.node)->data.list.values[i]=next;}
    if(old->kind==7) {
        dev_dynamic_node *r=(dev_dynamic_node*)result.node;r->kind=7;r->data.list.keys=(const char**)dev_dynamic_alloc((n+insert)*sizeof(char*));
        if(n)memcpy(r->data.list.keys,old->data.list.keys,n*sizeof(char*));if(insert)r->data.list.keys[n]=dev_dynamic_as_string(*path,file,line,col);
        /* Rebuild sorted keys for deterministic output. */
        result=dev_dynamic_object(n+insert,r->data.list.keys,r->data.list.values);
    }return result;
}
static inline void dev_dynamic_depth(dev_dynamic_value v,unsigned depth,const char *file,size_t line,size_t col) {
    if(depth>128)dev_dynamic_error("DON nesting limit exceeded",file,line,col);
    if(dev_dynamic_tag(v)>=6){const dev_dynamic_node *n=dev_dynamic_node_of(v);for(size_t i=0;i<n->data.list.len;i++)dev_dynamic_depth(n->data.list.values[i],depth+1,file,line,col);}
}
static inline dev_dynamic_value dev_dynamic_update(dev_dynamic_value root,size_t len,const dev_dynamic_value *path,dev_dynamic_value replacement,const char *op,const char *file,size_t line,size_t col) {
    dev_dynamic_value v=dev_dynamic_update_inner(root,len,path,replacement,op,file,line,col,0);dev_dynamic_depth(v,0,file,line,col);return v;
}
static inline dev_dynamic_value dev_dynamic_push(dev_dynamic_value v,dev_dynamic_value item,const char *file,size_t line,size_t col) {
    if(dev_dynamic_tag(v)!=6)dev_dynamic_error("DON push requires array",file,line,col);
    const dev_dynamic_node *n=dev_dynamic_node_of(v);size_t len=n->data.list.len;
    dev_dynamic_value *items=(dev_dynamic_value*)dev_dynamic_alloc((len+1)*sizeof(*items));if(len)memcpy(items,n->data.list.values,len*sizeof(*items));items[len]=item;
    dev_dynamic_value result=dev_dynamic_array(len+1,items);dev_dynamic_depth(result,0,file,line,col);return result;
}
static inline dev_dynamic_value dev_dynamic_remove(dev_dynamic_value v,dev_dynamic_value key,const char *file,size_t line,size_t col) {
    if(dev_dynamic_tag(v)!=7)dev_dynamic_error("DON remove requires object",file,line,col);
    size_t i=dev_dynamic_find(v,key,file,line,col);if(i==SIZE_MAX)return v;
    const dev_dynamic_node *n=dev_dynamic_node_of(v);size_t len=n->data.list.len;
    const char **keys=(const char**)dev_dynamic_alloc(len*sizeof(*keys));dev_dynamic_value *items=(dev_dynamic_value*)dev_dynamic_alloc(len*sizeof(*items));size_t j=0;
    for(size_t k=0;k<len;k++)if(k!=i){keys[j]=n->data.list.keys[k];items[j++]=n->data.list.values[k];}return dev_dynamic_object(j,keys,items);
}
typedef struct {char *data;size_t len,cap;} dev_dynamic_buffer;
static inline void dev_dynamic_append(dev_dynamic_buffer *b,const char *s,size_t n) {
    if(n>8*1024*1024-b->len){fputs("DON output exceeds 8 MiB\n",stderr);exit(1);}
    if(b->len+n+1>b->cap){size_t cap=(b->len+n+1)*2;char *data=(char*)realloc(b->data,cap);if(!data){fputs("DON serialization allocation failed\n",stderr);exit(1);}b->data=data;b->cap=cap;}
    memcpy(b->data+b->len,s,n);b->len+=n;b->data[b->len]=0;
}
static inline void dev_dynamic_quote(dev_dynamic_buffer *b,const char *s) {
    dev_dynamic_append(b,"\"",1);for(const unsigned char *p=(const unsigned char*)s;*p;p++){
        char text[7];const char *escape=NULL;
        switch(*p){case '"':escape="\\\"";break;case '\\':escape="\\\\";break;case '\n':escape="\\n";break;case '\r':escape="\\r";break;case '\t':escape="\\t";break;case '\b':escape="\\b";break;case '\f':escape="\\f";break;}
        if(escape)dev_dynamic_append(b,escape,2);else if(*p<32){snprintf(text,sizeof(text),"\\u%04x",*p);dev_dynamic_append(b,text,6);}else dev_dynamic_append(b,(const char*)p,1);
    }dev_dynamic_append(b,"\"",1);
}
static inline void dev_dynamic_indent(dev_dynamic_buffer *b,unsigned depth){dev_dynamic_append(b,"\n",1);for(unsigned i=0;i<depth;i++)dev_dynamic_append(b,"  ",2);}
static inline void dev_dynamic_render(dev_dynamic_buffer *b,dev_dynamic_value v,bool pretty,unsigned depth) {
    if(depth>128){fputs("DON nesting limit exceeded\n",stderr);exit(1);}
    const dev_dynamic_node *n=dev_dynamic_node_of(v);char number[64];int len;
    switch(dev_dynamic_tag(v)) {
        case 0:dev_dynamic_append(b,"null",4);break;
        case 1:dev_dynamic_append(b,n->data.boolean?"true":"false",n->data.boolean?4:5);break;
        case 2:len=snprintf(number,sizeof(number),"%lld",(long long)n->data.integer);dev_dynamic_append(b,number,(size_t)len);break;
        case 3:len=snprintf(number,sizeof(number),"%llu",(unsigned long long)n->data.unsigned_integer);dev_dynamic_append(b,number,(size_t)len);break;
        case 4:len=snprintf(number,sizeof(number),"%.17g",n->data.number);dev_dynamic_append(b,number,(size_t)len);if(!strchr(number,'.')&&!strchr(number,'e')&&!strchr(number,'E'))dev_dynamic_append(b,".0",2);break;
        case 5:dev_dynamic_quote(b,n->data.string);break;
        default:{bool object=n->kind==7;dev_dynamic_append(b,object?"{":"[",1);for(size_t i=0;i<n->data.list.len;i++){
            if(i)dev_dynamic_append(b,",",1);if(pretty)dev_dynamic_indent(b,depth+1);
            if(object){dev_dynamic_quote(b,n->data.list.keys[i]);dev_dynamic_append(b,pretty?": ":":",pretty?2:1);}
            dev_dynamic_render(b,n->data.list.values[i],pretty,depth+1);
        }if(pretty&&n->data.list.len)dev_dynamic_indent(b,depth);dev_dynamic_append(b,object?"}":"]",1);}
    }
}
static inline const char *dev_dynamic_serialize(dev_dynamic_value v,bool pretty,const char *file,size_t line,size_t col) {
    dev_dynamic_depth(v,0,file,line,col);dev_dynamic_buffer b={0};dev_dynamic_render(&b,v,pretty,0);const char *s=dev_dynamic_copy_string(b.data);free(b.data);return s;
}
static inline const char *dev_dynamic_text(dev_dynamic_value v,bool print,const char *file,size_t line,size_t col) {
    if(!print&&v.boxed)dev_dynamic_error("interpolation supports scalar DON fields only",file,line,col);
    if(!v.boxed&&dev_dynamic_tag(v)==5)return dev_dynamic_node_of(v)->data.string;
    if(!v.boxed&&dev_dynamic_tag(v)==4) {char text[64];snprintf(text,sizeof(text),"%.17g",dev_dynamic_node_of(v)->data.number);return dev_dynamic_copy_string(text);}
    return dev_dynamic_serialize(v,false,file,line,col);
}
static inline bool dev_dynamic_bare_key(const char *s) {
    const unsigned char *p=(const unsigned char*)s;
    if(!((*p>='a'&&*p<='z')||(*p>='A'&&*p<='Z')||*p=='_'||*p=='$'))return false;
    for(p++;*p;p++)if(!((*p>='a'&&*p<='z')||(*p>='A'&&*p<='Z')||(*p>='0'&&*p<='9')||*p=='_'||*p=='-'||*p=='$'))return false;
    return true;
}
static inline void dev_dynamic_don_render(dev_dynamic_buffer *b,dev_dynamic_value v,unsigned depth) {
    if(depth>128){fputs("DON nesting limit exceeded\n",stderr);exit(1);}
    const dev_dynamic_node *n=dev_dynamic_node_of(v);int tag=dev_dynamic_tag(v);
    if(tag==5) {
        /* DON double quotes must escape interpolation starters in literal data. */
        dev_dynamic_buffer quoted={0};dev_dynamic_quote(&quoted,n->data.string);
        for(size_t i=0;i<quoted.len;i++){if(quoted.data[i]=='$'&&quoted.data[i+1]=='{')dev_dynamic_append(b,"\\",1);dev_dynamic_append(b,quoted.data+i,1);}free(quoted.data);return;
    }
    if((tag==6||tag==7)&&n->data.list.len) {
        bool object=tag==7;dev_dynamic_append(b,object?"{":"[",1);
        for(size_t i=0;i<n->data.list.len;i++) {
            dev_dynamic_indent(b,depth+1);
            if(object) {const char *key=n->data.list.keys[i];if(dev_dynamic_bare_key(key))dev_dynamic_append(b,key,strlen(key));else dev_dynamic_quote(b,key);dev_dynamic_append(b,": ",2);}
            dev_dynamic_don_render(b,n->data.list.values[i],depth+1);
        }dev_dynamic_indent(b,depth);dev_dynamic_append(b,object?"}":"]",1);return;
    }
    dev_dynamic_render(b,v,false,depth);
}
static inline const char *dev_dynamic_don_serialize(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    dev_dynamic_depth(v,0,file,line,col);dev_dynamic_buffer b={0};dev_dynamic_don_render(&b,v,0);const char *s=dev_dynamic_copy_string(b.data);free(b.data);return s;
}

/* Strict JSON decoding for native data exchange; DON document evaluation stays
   in the source library. Parsing failures are returned to valid(), not thrown. */
typedef struct {const unsigned char *p;bool failed,allow_nul;const char *reason;} dev_dynamic_parser;
static inline void dev_dynamic_parse_space(dev_dynamic_parser *p) {while(*p->p==' '||*p->p=='\t'||*p->p=='\r'||*p->p=='\n')p->p++;}
static inline unsigned dev_dynamic_hex(unsigned char c) {if(c>='0'&&c<='9')return c-'0';if(c>='a'&&c<='f')return c-'a'+10;if(c>='A'&&c<='F')return c-'A'+10;return 16;}
static inline unsigned dev_dynamic_parse_hex(dev_dynamic_parser *p) {
    unsigned n=0;for(unsigned i=0;i<4;i++){unsigned c=dev_dynamic_hex(*p->p);if(c==16){p->failed=true;return 0;}n=n*16+c;p->p++;}return n;
}
static inline void dev_dynamic_utf8(dev_dynamic_buffer *b,unsigned n) {
    char out[4];size_t len;if(n<128){out[0]=(char)n;len=1;}else if(n<2048){out[0]=(char)(0xc0|(n>>6));out[1]=(char)(0x80|(n&63));len=2;}else if(n<65536){out[0]=(char)(0xe0|(n>>12));out[1]=(char)(0x80|((n>>6)&63));out[2]=(char)(0x80|(n&63));len=3;}else {out[0]=(char)(0xf0|(n>>18));out[1]=(char)(0x80|((n>>12)&63));out[2]=(char)(0x80|((n>>6)&63));out[3]=(char)(0x80|(n&63));len=4;}dev_dynamic_append(b,out,len);
}
static inline const char *dev_dynamic_parse_string(dev_dynamic_parser *p) {
    dev_dynamic_buffer b={0};p->p++;
    while(*p->p&&*p->p!='"'&&!p->failed) {
        unsigned char c=*p->p++;
        if(c<32){p->failed=true;break;}
        if(c!='\\'){dev_dynamic_append(&b,(const char*)&c,1);continue;}
        c=*p->p;if(!c){p->failed=true;break;}p->p++;
        switch(c){case '"':case '\\':case '/':dev_dynamic_append(&b,(const char*)&c,1);break;
            case 'b':dev_dynamic_append(&b,"\b",1);break;case 'f':dev_dynamic_append(&b,"\f",1);break;case 'n':dev_dynamic_append(&b,"\n",1);break;case 'r':dev_dynamic_append(&b,"\r",1);break;case 't':dev_dynamic_append(&b,"\t",1);break;
            case 'u': {unsigned n=dev_dynamic_parse_hex(p);if(p->failed)break;
                if(n>=0xd800&&n<=0xdbff){if(p->p[0]!='\\'||p->p[1]!='u'){p->failed=true;break;}p->p+=2;unsigned low=dev_dynamic_parse_hex(p);if(low<0xdc00||low>0xdfff){p->failed=true;break;}n=0x10000+((n-0xd800)<<10)+(low-0xdc00);}
                else if(n>=0xdc00&&n<=0xdfff){p->failed=true;break;}
                if(!n&&!p->allow_nul){p->failed=true;p->reason="embedded NUL is unsupported in native C strings";break;}
                dev_dynamic_utf8(&b,n);break;
            }
            default:p->failed=true;
        }
    }
    if(*p->p!='"')p->failed=true;else p->p++;
    const char *s=dev_dynamic_copy_string(b.data?b.data:"");free(b.data);return s;
}
static dev_dynamic_value dev_dynamic_parse_value(dev_dynamic_parser *p,unsigned depth);
static inline dev_dynamic_value dev_dynamic_parse_list(dev_dynamic_parser *p,unsigned depth,bool object) {
    p->p++;dev_dynamic_parse_space(p);unsigned char end=object?'}':']';size_t count=0,cap=0;
    dev_dynamic_value *items=NULL;const char **keys=NULL;
    while(!p->failed&&*p->p!=end) {
        const char *key=NULL;
        if(object){if(*p->p!='"'){p->failed=true;break;}key=dev_dynamic_parse_string(p);dev_dynamic_parse_space(p);if(*p->p!=':'){p->failed=true;break;}p->p++;}
        dev_dynamic_value item=dev_dynamic_parse_value(p,depth+1);if(p->failed)break;
        size_t at=count;
        if(object)for(size_t i=0;i<count;i++)if(!strcmp(keys[i],key)){at=i;break;}
        if(at==count){if(count==cap){cap=cap?cap*2:8;dev_dynamic_value *new_items=(dev_dynamic_value*)realloc(items,cap*sizeof(*items));const char **new_keys=object?(const char**)realloc(keys,cap*sizeof(*keys)):NULL;if(!new_items||(object&&!new_keys)){fputs("JSON allocation failed\n",stderr);exit(1);}items=new_items;keys=new_keys;}count++;}
        items[at]=item;if(object)keys[at]=key;dev_dynamic_parse_space(p);
        if(*p->p==end)break;
        if(*p->p!=','){p->failed=true;break;}p->p++;dev_dynamic_parse_space(p);if(*p->p==end){p->failed=true;break;}
    }
    if(*p->p!=end)p->failed=true;else p->p++;
    dev_dynamic_value result=object?dev_dynamic_object(count,keys,items):dev_dynamic_array(count,items);free(items);free(keys);return result;
}
static dev_dynamic_value dev_dynamic_parse_value(dev_dynamic_parser *p,unsigned depth) {
    dev_dynamic_parse_space(p);if(depth>128){p->failed=true;p->reason="JSON nesting limit exceeded";return (dev_dynamic_value){NULL,1};}
    switch(*p->p){case '"':return dev_dynamic_string(dev_dynamic_parse_string(p));case '{':return dev_dynamic_parse_list(p,depth,true);case '[':return dev_dynamic_parse_list(p,depth,false);
        case 't':if(!strncmp((const char*)p->p,"true",4)){p->p+=4;return dev_dynamic_bool(true);}break;
        case 'f':if(!strncmp((const char*)p->p,"false",5)){p->p+=5;return dev_dynamic_bool(false);}break;
        case 'n':if(!strncmp((const char*)p->p,"null",4)){p->p+=4;return (dev_dynamic_value){NULL,1};}break;
        default:{const unsigned char *start=p->p;bool negative=false,floating=false;if(*p->p=='-'){negative=true;p->p++;}
            if(*p->p=='0')p->p++;else if(*p->p>='1'&&*p->p<='9'){while(*p->p>='0'&&*p->p<='9')p->p++;}else {p->failed=true;break;}
            if(*p->p=='.'){floating=true;p->p++;if(*p->p<'0'||*p->p>'9'){p->failed=true;break;}while(*p->p>='0'&&*p->p<='9')p->p++;}
            if(*p->p=='e'||*p->p=='E'){floating=true;p->p++;if(*p->p=='+'||*p->p=='-')p->p++;if(*p->p<'0'||*p->p>'9'){p->failed=true;break;}while(*p->p>='0'&&*p->p<='9')p->p++;}
            size_t len=(size_t)(p->p-start);char *text=(char*)malloc(len+1);if(!text){fputs("JSON allocation failed\n",stderr);exit(1);}memcpy(text,start,len);text[len]=0;
            if(!floating){uint64_t n=0;bool overflow=false;const char *s=text+negative;while(*s){unsigned digit=(unsigned)(*s++-'0');if(n>(UINT64_MAX-digit)/10){overflow=true;break;}n=n*10+digit;}
                if(!overflow&&(!negative||n<=UINT64_C(9223372036854775808))){free(text);if(negative)return dev_dynamic_int((int64_t)(UINT64_C(0)-n));return n<=INT64_MAX?dev_dynamic_int((int64_t)n):dev_dynamic_uint(n);}
            }
            double n=strtod(text,NULL);free(text);if(!isfinite(n)){p->failed=true;break;}return dev_dynamic_float(n);
        }
    }p->failed=true;return (dev_dynamic_value){NULL,1};
}
static inline dev_dynamic_value dev_dynamic_parse(const char *text,const char *file,size_t line,size_t col) {
    if(strlen(text)>8*1024*1024)dev_dynamic_error("JSON source exceeds 8 MiB",file,line,col);
    dev_dynamic_parser p={(const unsigned char*)text,false,false,"invalid JSON"};dev_dynamic_value v=dev_dynamic_parse_value(&p,0);dev_dynamic_parse_space(&p);if(p.failed||*p.p)dev_dynamic_error(p.reason,file,line,col);return v;
}
static inline bool dev_dynamic_valid(const char *text) {
    if(strlen(text)>8*1024*1024)return false;dev_dynamic_parser p={(const unsigned char*)text,false,true,"invalid JSON"};(void)dev_dynamic_parse_value(&p,0);dev_dynamic_parse_space(&p);return !p.failed&&!*p.p;
}

static inline size_t dev_dynamic_array_len(dev_dynamic_value v,const char *file,size_t line,size_t col) {
    const dev_dynamic_node *node=(const dev_dynamic_node *)v.node;
    if(!node || node->kind!=6) dev_dynamic_error("collection iteration requires an array",file,line,col);
    return node->data.list.len;
}

static inline bool dev_dynamic_is_array(dev_dynamic_value v) { return dev_dynamic_tag(v)==6; }
