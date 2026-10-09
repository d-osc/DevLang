from source_text import unsafe_fixture
"""Run source-only FFI, with automatic preparation delegated to devc."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--bin-dir',required=True)
    folder=Path(parser.parse_args().bin_dir).resolve()
    suffix='.exe' if os.name=='nt' else ''
    count=0
    with tempfile.TemporaryDirectory(prefix='dev-auto-native-') as d:
        root=Path(d)
        entry=root/'main.dev'
        entry.write_text('extern fn device_add(a i32,b i32) i32\nunsafe { print(device_add(19,23)) }')
        source=root/'device.c'
        source.write_text('#include <stdint.h>\n#include "include/config.h"\nextern int32_t helper(int32_t,int32_t);\nint32_t device_add(int32_t a,int32_t b) { return helper(a,b)+BONUS; }')
        (root/'helper.c').write_text('#include <stdint.h>\nint32_t helper(int32_t a,int32_t b) { return a+b; }')
        (root/'include').mkdir()
        header=root/'include/config.h'
        header.write_text('#define BONUS 0\n')
        clean=dict(os.environ)
        clean.pop('DEV_CC',None)
        no_compiler=dict(clean,PATH='')
        def run(expected=None,error=None,env=None):
            nonlocal count
            result=subprocess.run([str(folder/('d'+suffix)),str(entry)],cwd=root,
                                  capture_output=True,text=True,env=clean if env is None else env,timeout=60)
            if error:
                assert result.returncode!=0 and error in result.stderr,(result.stdout,result.stderr)
            else:
                assert result.returncode==0 and result.stdout==expected,(result.stdout,result.stderr)
            count+=1
        assert not list(root.glob('*.dll')) and not list(root.glob('*.so'))
        run('42\n')
        cache=root/'.dev-cache/native'
        libraries=list(cache.glob('*.dll' if os.name=='nt' else '*.so'))
        assert len(libraries)==1,libraries
        stamp=libraries[0].stat().st_mtime_ns
        run('42\n',env=no_compiler)
        assert libraries[0].stat().st_mtime_ns==stamp
        libraries[0].write_bytes(b'corrupt native cache')
        run('42\n')
        header.write_text('#define BONUS 1\n')
        run('43\n')
        source.write_text(source.read_text().replace('+BONUS','+BONUS+1'))
        run('44\n')
        run('44\n',env=no_compiler)
        previous=list(cache.glob('*.dll' if os.name=='nt' else '*.so'))
        # Preserve the symbol name so the runtime recognizes this C implementation.
        source.write_text('invalid C code /* device_add */')
        run(error='automatic C dependency preparation failed')
        assert set(previous)==set(cache.glob('*.dll' if os.name=='nt' else '*.so'))
        source.write_text('#include <stdint.h>\nint32_t device_add(int32_t a,int32_t b) { return a+b+10; }')
        run(error='Clang/GCC',env=no_compiler)
        run('52\n')
        # Headers and C source live next to an imported declaration module.
        imported=root/'modules'
        imported.mkdir()
        (imported/'native.dev').write_text('extern fn device_add(a i32,b i32) i32')
        source.rename(imported/'device.c')
        entry.write_text('use "modules/native.dev" as native\nunsafe { print(native.device_add(19,23)) }')
        run('52\n')
        run('52\n',env=no_compiler)
    print(f'PASS: {count} source-only FFI scenarios; no manual build, unannotated C exports, multiple C files, header/source invalidation, warm execution without a C backend and failed-build preservation')

if __name__=='__main__':
    main()
