from source_text import unsafe_fixture
"""Build C ABI fixtures once, then interpret Dev with an empty compiler PATH."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

C_SOURCE=r'''
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <string.h>
#include <stdio.h>
#ifdef _WIN32
#define API __declspec(dllexport)
#else
#define API
#endif
API int8_t ffi_i8(int8_t n) { return n; }
API uint8_t ffi_u8(uint8_t n) { return n; }
API int16_t ffi_i16(int16_t n) { return n; }
API uint16_t ffi_u16(uint16_t n) { return n; }
API int32_t ffi_i32(int32_t n) { return n; }
API uint32_t ffi_u32(uint32_t n) { return n; }
API int64_t ffi_i64(int64_t n) { return n; }
API uint64_t ffi_u64(uint64_t n) { return n; }
API size_t ffi_size(size_t n) { return n; }
API intptr_t ffi_signed_size(intptr_t n) { return n; }
API bool ffi_bool(bool b) { return !b; }
API float ffi_f32(float n) { return n + 0.5f; }
API double ffi_f64(double n) { return n + 0.5; }
API double ffi_mixed(int8_t a, double b, uint16_t c, float d, int64_t e, double f,
                     int32_t g, double h, uint64_t i, double j) { return a+b+c+d+e+f+g+h+i+j; }
API size_t ffi_len(const char *s) { return strlen(s); }
API const char *ffi_echo(const char *s) { return s; }
API const char *ffi_null_str(void) { return NULL; }
API const char *ffi_bad_str(void) { return "\xff"; }
static int32_t token=42;
API int32_t *ffi_pointer(void) { return &token; }
API int32_t ffi_read(int32_t *p) { return *p; }
API void *ffi_null(void) { return NULL; }
API void ffi_write(const char *s) { puts(s); fflush(stdout); }
API int32_t device_add(int32_t a, int32_t b) { return a+b; }
'''

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--bin-dir',required=True)
    parser.add_argument('--cc',default='clang' if os.name=='nt' else 'cc')
    args=parser.parse_args()
    folder=Path(args.bin_dir).resolve()
    suffix='.exe' if os.name=='nt' else ''
    count=0
    with tempfile.TemporaryDirectory(prefix='dev-ffi-') as d:
        root=Path(d)
        source=root/'native.c'
        source.write_text(C_SOURCE)
        library=root/('ภาษาไทย library.dll' if os.name=='nt' else 'ภาษาไทย library.so')
        flags=['-shared','-O2']+([] if os.name=='nt' else ['-fPIC'])
        subprocess.run([args.cc,*flags,str(source),'-o',str(library)],cwd=root,check=True,capture_output=True)
        env=dict(os.environ,PATH='',CC='missing',DEV_CC='missing')
        entry=root/'main.dev'
        def run(code,expected=None,error=None,libraries=None,tool='devrun',status=0):
            nonlocal count
            entry.write_text(unsafe_fixture(code),encoding='utf-8')
            libs=[library] if libraries is None else libraries
            command=[str(folder/(tool+suffix)),str(entry)]
            for lib in libs:command.extend(['--ffi-lib',str(lib)])
            result=subprocess.run(command,cwd=root,env=env,capture_output=True,text=True,encoding='utf-8',timeout=15)
            if error:
                assert result.returncode!=0 and error in result.stderr,(code,result.stdout,result.stderr)
            else:
                assert (result.returncode,result.stdout)==(status,expected),(code,result.stdout,result.stderr)
            count+=1
        for t,n in [('i8','-128'),('u8','255'),('i16','-32768'),('u16','65535'),('i32','-2147483648'),
                    ('u32','4294967295'),('i64','-9223372036854775808'),('u64','18446744073709551615')]:
            run(f'extern fn ffi_{t}(n {t}) {t}\nprint(ffi_{t}({n}))',n+'\n')
        run('extern fn ffi_size(n usize) usize\nextern fn ffi_signed_size(n isize) isize\nprint(ffi_size(42)); print(ffi_signed_size(-42))','42\n-42\n')
        run('extern fn ffi_bool(b bool) bool\nprint(ffi_bool(false)); print(ffi_bool(true))','true\nfalse\n')
        for t in ('f32','f64'):
            run(f'extern fn ffi_{t}(n {t}) {t}\nprint(ffi_{t}(1.25))','1.75\n')
        run('extern fn ffi_mixed(a i8,b f64,c u16,d f32,e i64,f f64,g i32,h f64,i u64,j f64) f64\nprint(ffi_mixed(1,2,3,4,5,6,7,8,9,10))','55\n')
        run('extern fn ffi_len(s str) usize\nextern fn ffi_echo(s str) str\nprint(ffi_len("ภาษาไทย")); print(ffi_echo("ภาษาไทย"))','21\nภาษาไทย\n')
        run('extern fn ffi_pointer() *i32\nextern fn ffi_read(p *i32) i32\nextern fn ffi_null() *void\nlet p=ffi_pointer(); print(p!=null); print(ffi_read(p)); print(ffi_null()==null); print((p as usize)>0)','true\n42\ntrue\ntrue\n')
        run('extern fn malloc(n usize) *void\nextern fn free(p *void)\nlet p=malloc(16); print(p!=null); free(p)','true\n',libraries=[])
        run('extern fn puts(s str) i32\nputs("standard C"); print(42)','standard C\n42\n',libraries=[])
        run('extern fn ffi_write(s str)\nffi_write("native"); print("runtime")','native\nruntime\n')
        run('extern fn ffi_i32(n i32) i32\nlet n i32=0; let sum i32=0; while n<1000 {sum+=ffi_i32(n); n+=1}; print(sum)','499500\n')
        fixture=(Path(__file__).resolve().parents[1]/'examples/ffi/main.dev').read_text()
        run(fixture,'Calling C directly\n42\n',tool='d')
        run(fixture,'Calling C directly\n42\n',tool='d',libraries=[])
        run('extern fn ffi_i32(n i32) i32\nprint(ffi_i32(42))','42\n',libraries=[])
        invalid=root/('a_bad.dll' if os.name=='nt' else 'a_bad.so')
        invalid.write_bytes(b'not a shared library')
        run(fixture,'Calling C directly\n42\n',tool='d',libraries=[])
        run('extern fn symbol_not_in_any_library()\nsymbol_not_in_any_library()',error='adjacent libraries failed to load',libraries=[])
        invalid.unlink()
        run('extern fn ffi_i32(n i32) i32\nreturn ffi_i32(37)','',status=37)
        for code,error in [
            ('extern fn missing_symbol() i32\nmissing_symbol()','native symbol'),
            ('extern fn ffi_i32(n i32) i32\nffi_i32(true)','expected i32'),
            ('extern fn ffi_i32(n i32) i32\nffi_i32()','wrong argument count'),
            ('extern fn ffi_echo(s str) str\nffi_echo("a\\0b")','NUL'),
            ('extern fn ffi_null_str() str\nffi_null_str()','returned null'),
            ('extern fn ffi_bad_str() str\nffi_bad_str()','UTF-8'),
            ('extern fn unsupported(a [i32; 2])\nunsupported([1,2])','unsupported native FFI'),
            ('extern fn ffi_i32(n i32) i32\nextern fn ffi_i32(n f64) f64','conflicting native signatures'),
        ]:
            run(code,error=error)
        run('print(42)',error='cannot load native library',libraries=[root/'absent.dll'])
        run('extern fn ffi_i32(n i32) i32\nprint(ffi_i32(42))','42\n',libraries=[library,library])
        (root/'native.dev').write_text('extern fn ffi_i32(n i32) i32')
        run('use native\nprint(native.ffi_i32(42))','42\n')
        imported=root/'modules'
        imported.mkdir()
        (imported/'native.dev').write_text('extern fn ffi_i32(n i32) i32')
        import shutil
        moved=imported/library.name
        shutil.move(str(library),moved)
        run('use "modules/native.dev" as native\nprint(native.ffi_i32(42))','42\n',libraries=[])
        # Test a missing implementation, rather than automatic C preparation.
        saved=source.with_suffix('.c.saved')
        source.rename(saved)
        try:
            run(fixture,error='native symbol',libraries=[])
        finally:
            saved.rename(source)
        shutil.move(str(moved),library)
        assert not (root/'out').exists() and not (root/'.dev-cache').exists()
    print(f'PASS: {count} native FFI scenarios; C ABI scalars, floats, strings, pointers, mixed register/stack args, errors and empty compiler PATH')

if __name__=='__main__':
    main()
