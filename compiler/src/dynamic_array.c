/* Text, equality and flattening for dynamic array elements. */
static const char *dev_array_dynamic_text(dev_dynamic_value v,const char *p,size_t l,size_t c,unsigned depth){
 if(depth>128)dev_dynamic_error("array text nesting limit exceeded",p,l,c);int tag=dev_dynamic_tag(v);
 if(!tag)return "";if(tag==8)return "[Function]";
 if(v.node&&dev_dynamic_node_of(v)->kind==9){dev_dynamic_value r=dev_object_method(v,"toString",0,NULL,p,l,c);if(dev_dynamic_tag(r)>=6)dev_dynamic_error("object toString must return a scalar",p,l,c);if(!dev_dynamic_tag(r))return "null";return dev_array_dynamic_text(r,p,l,c,depth+1);}
 if(tag==7)return "[object Object]";
 if(tag==6){const dev_dynamic_node *n=dev_dynamic_node_of(v);dev_dynamic_buffer b={0};for(size_t i=0;i<n->data.list.len;i++){if(i)dev_dynamic_append(&b,",",1);{const char *text=dev_array_dynamic_text(n->data.list.values[i],p,l,c,depth+1);dev_dynamic_append(&b,text,strlen(text));};}const char *s=dev_dynamic_copy_string(b.data?b.data:"");free(b.data);return s;}
 return dev_dynamic_text(dev_dynamic_unboxed(v),false,p,l,c);
}
static bool dev_array_dynamic_equal(dev_dynamic_value a,dev_dynamic_value b,bool nan_equal,const char *p,size_t l,size_t c){
 int at=dev_dynamic_tag(a),bt=dev_dynamic_tag(b);if(at>=2&&at<=4&&bt>=2&&bt<=4){
 const dev_dynamic_node *an=dev_dynamic_node_of(a),*bn=dev_dynamic_node_of(b);
 if(at<4&&bt<4){if(at==2&&bt==2)return an->data.integer==bn->data.integer;if(at==3&&bt==3)return an->data.unsigned_integer==bn->data.unsigned_integer;if(at==2)return an->data.integer>=0&&(uint64_t)an->data.integer==bn->data.unsigned_integer;return bn->data.integer>=0&&an->data.unsigned_integer==(uint64_t)bn->data.integer;}
 if(at==4&&bt==4){double x=an->data.number,y=bn->data.number;return x==y||(nan_equal&&isnan(x)&&isnan(y));}
 const dev_dynamic_node *integer=at==4?bn:an;int it=at==4?bt:at;double f=at==4?an->data.number:bn->data.number;
 if(!isfinite(f)||floor(f)!=f)return false;if(it==2)return f>=-9223372036854775808.0&&f<9223372036854775808.0&&(int64_t)f==integer->data.integer;return f>=0.0&&f<18446744073709551616.0&&(uint64_t)f==integer->data.unsigned_integer;
 }
 if(at!=bt)return false;if(!at)return true;if(at==1)return dev_dynamic_node_of(a)->data.boolean==dev_dynamic_node_of(b)->data.boolean;if(at==5)return !strcmp(dev_dynamic_node_of(a)->data.string,dev_dynamic_node_of(b)->data.string);return a.node==b.node;
}
static int64_t dev_array_dynamic_compare(dev_dynamic_value a,dev_dynamic_value b,const char *p,size_t l,size_t c){const char *left=dev_dynamic_tag(a)?dev_array_dynamic_text(a,p,l,c,0):"null";const char *right=dev_dynamic_tag(b)?dev_array_dynamic_text(b,p,l,c,0):"null";return strcmp(left,right);}
static void dev_array_dynamic_flat_inner(dev_dynamic_value v,unsigned depth,dev_dynamic_value **items,size_t *count,size_t *capacity,const char *p,size_t l,size_t c){
 if(depth&&dev_dynamic_tag(v)==6){const dev_dynamic_node *n=dev_dynamic_node_of(v);for(size_t i=0;i<n->data.list.len;i++)dev_array_dynamic_flat_inner(n->data.list.values[i],depth-1,items,count,capacity,p,l,c);return;}
 if(*count==1000000)dev_dynamic_error("flattened array exceeds 1000000 elements",p,l,c);if(*count==*capacity){size_t next=*capacity?*capacity*2:16;dev_dynamic_value *grown=realloc(*items,next*sizeof(*grown));if(!grown)dev_dynamic_error("array allocation failed",p,l,c);*items=grown;*capacity=next;}(*items)[(*count)++]=dev_object_box_data(v);
}
static dev_dynamic_value dev_array_dynamic_flat(dev_dynamic_value v,unsigned depth,const char *p,size_t l,size_t c){dev_dynamic_value *items=NULL;size_t count=0,capacity=0;dev_array_dynamic_flat_inner(v,depth,&items,&count,&capacity,p,l,c);dev_dynamic_value out=dev_dynamic_array(count,items);free(items);return out;}
