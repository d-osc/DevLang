"""Call the DevLang C ABI through a shared adapter library."""
import ctypes as C
from pathlib import Path
import sys

lib = C.CDLL(str(Path(sys.argv[1]).resolve()))
lib.bridge_add.argtypes = [C.c_int32, C.c_int32]
lib.bridge_add.restype = C.c_int32
lib.bridge_scale.argtypes = [C.c_double, C.c_double]
lib.bridge_scale.restype = C.c_double
lib.bridge_positive.argtypes = [C.c_int32]
lib.bridge_positive.restype = C.c_bool
lib.bridge_name.argtypes = []
lib.bridge_name.restype = C.c_char_p
print(lib.bridge_add(20, 22))
print(lib.bridge_scale(2.5, 5.0))
print(str(lib.bridge_positive(7)).lower())
print(str(lib.bridge_positive(-7)).lower())
# c_char_p yields bytes from a borrowed static string; do not free it.
print(lib.bridge_name().decode('utf-8'))
