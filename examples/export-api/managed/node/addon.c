#include <node_api.h>
#include <stdint.h>
#include "../../devmath.h"

static int arguments(napi_env env, napi_callback_info info, size_t count, napi_value *values) {
    size_t received = count;
    if (napi_get_cb_info(env, info, &received, values, NULL, NULL) != napi_ok || received != count) {
        napi_throw_type_error(env, NULL, "wrong argument count"); return 0;
    }
    return 1;
}
static int integer(napi_env env, napi_value value, int32_t *number) {
    double v;
    if (napi_get_value_double(env, value, &v) != napi_ok ||
        !(v >= INT32_MIN && v <= INT32_MAX) || v != (int32_t)v) {
        napi_throw_type_error(env, NULL, "expected an i32 integer"); return 0;
    }
    *number = (int32_t)v; return 1;
}
static napi_value add(napi_env env, napi_callback_info info) {
    napi_value values[2], result; int32_t a, b;
    if (!arguments(env, info, 2, values) || !integer(env, values[0], &a) || !integer(env, values[1], &b)) return NULL;
    int64_t sum = (int64_t)a + b;
    if (sum < INT32_MIN || sum > INT32_MAX) { napi_throw_range_error(env, NULL, "i32 overflow"); return NULL; }
    napi_create_int32(env, devmath_add(a,b), &result); return result;
}
static napi_value scale(napi_env env, napi_callback_info info) {
    napi_value values[2], result; double a, b;
    if (!arguments(env, info, 2, values)) return NULL;
    if (napi_get_value_double(env, values[0], &a) != napi_ok || napi_get_value_double(env, values[1], &b) != napi_ok) {
        napi_throw_type_error(env, NULL, "expected two numbers"); return NULL;
    }
    napi_create_double(env, devmath_scale(a,b), &result); return result;
}
static napi_value positive(napi_env env, napi_callback_info info) {
    napi_value values[1], result; int32_t value;
    if (!arguments(env, info, 1, values) || !integer(env, values[0], &value)) return NULL;
    napi_get_boolean(env, devmath_positive(value), &result); return result;
}
static napi_value name(napi_env env, napi_callback_info info) {
    (void)info; napi_value result;
    napi_create_string_utf8(env, devmath_name(), NAPI_AUTO_LENGTH, &result); return result;
}
static napi_value init(napi_env env, napi_value exports) {
    napi_property_descriptor properties[] = {
        {"add",NULL,add,NULL,NULL,NULL,napi_default,NULL},
        {"scale",NULL,scale,NULL,NULL,NULL,napi_default,NULL},
        {"positive",NULL,positive,NULL,NULL,NULL,napi_default,NULL},
        {"name",NULL,name,NULL,NULL,NULL,napi_default,NULL}
    };
    if (napi_define_properties(env, exports, 4, properties) != napi_ok) return NULL;
    return exports;
}
NAPI_MODULE(devmath, init)
