"""Validate real launcher aliases, independent tools and argument forwarding."""
from source_text import explicit_entry
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bin-dir', required=True)
    binaries = Path(parser.parse_args().bin_dir).resolve()
    suffix = '.exe' if os.name == 'nt' else ''
    count = 0
    with tempfile.TemporaryDirectory(prefix='dev-cli-') as directory:
        root = Path(directory)
        runtime = root/'runtime only'
        compiler = root/'compiler only'
        for folder, tool in ((runtime,'devrun'),(compiler,'devc')):
            folder.mkdir()
            for name in ('d',tool):
                shutil.copy2(binaries/(name+suffix),folder/(name+suffix))
        source = root/'main file.dev'
        (root/'math.dev').write_text(explicit_entry('fn answer() i64 { return 42 }', path=root/'math.dev'))
        source.write_text(explicit_entry('use math\nuse "std/args"\nuse "std/io"\nfn main() { print(math.answer()); print(args.get(0)); print(io.read_line()) }', path=source))
        empty_path = dict(os.environ,PATH='',CC='missing-compiler',DEV_CC='missing-compiler')
        def run(folder, args, expected=None, env=None, stdin='', code=0, error=None):
            nonlocal count
            result = subprocess.run([str(folder/('d'+suffix)),*map(str,args)],cwd=root,env=env,input=stdin,
                                    capture_output=True,text=True,encoding='utf-8',timeout=60)
            assert result.returncode == code,(args,result.stdout,result.stderr)
            if expected is not None:
                assert result.stdout == expected,(args,result.stdout,result.stderr)
            if error:
                assert error in result.stderr,result.stderr
            count += 1
            return result
        for selector in ([],['run'],['--run'],['-r']):
            run(runtime,[*selector,source,'--','hello world'], '42\nhello world\ninput\n',env=empty_path,stdin='input\n')
        assert not (root/'out').exists() and not (root/'.dev-cache').exists()
        run(runtime,['-e','return 37'],env=empty_path,code=37)
        run(runtime,['-e','print(6 * 7)'],'42\n',env=empty_path)
        source.write_text(explicit_entry('use math\nfn main() { print(math.answer()) }', path=source))
        for selector in ('build','--build','-b','compiler','--compiler','-c'):
            output = root/(selector.replace('-','')+' program'+suffix)
            run(compiler,[selector,source,'--release','-o',output])
            result = subprocess.run([str(output)],capture_output=True,text=True)
            assert result.returncode == 0 and result.stdout=='42\n',result.stderr
        run(runtime,['build',source],code=1,error='devc')
        run(compiler,[source],code=1,error='devrun')
        run(runtime,['-r'],code=1,error='expected a .dev')
        run(runtime,['--unknown',source],code=1,error='unknown option')
        run(runtime,['--help'],env=empty_path)
        run(runtime,['--version'],'d 0.1.0\n',env=empty_path)
        for flag in ('-v','-V'):
            run(runtime,[flag],'d 0.1.0\n',env=empty_path)
        for command in ('run','build','compiler','check','emit'):
            result = run(runtime,[command,'--help'],env=empty_path)
            assert '--output' in result.stdout and '--cwd' in result.stdout
        run(runtime,[source,'-h'],env=empty_path)
        run(runtime,['--eval','print("--help")'],'--help\n',env=empty_path)
        run(runtime,['run','--eval','print(42)'],'42\n',env=empty_path)
        for mode in ('auto','ast'):
            run(runtime,['run','--eval','print(42)','--engine',mode],'42\n',env=empty_path)
        run(runtime,['-e','print(42)','--engine','unknown'],code=1,error='--engine needs')
        run(runtime,['-e','print(42)','--engine'],code=1,error='option needs')
        for flags in (['--timings'],[]):
            result = run(runtime,[*flags,'-e','print(42)'],'42\n',env=empty_path)
            assert ('runtime: load ' in result.stderr) == bool(flags)
        run(runtime,['-e','use'],env=empty_path,code=1,error='expected module')
        run(runtime,['--cwd',root,source.name],'42\n',env=empty_path)
        run(runtime,['-C',root,'-r',source.name],'42\n',env=empty_path)
        run(runtime,['-C',root/'absent',source.name],code=1,error='working directory')
        run(runtime,['-C'],code=1,error='needs a directory')
        run(runtime,['--eval'],code=1,error='option needs a value')
        result = run(compiler,['--check',source])
        assert 'checked 2 modules' in result.stdout
        generated = root/'generated C'
        run(compiler,['--emit',source,'--output',generated])
        assert list(generated.rglob('*.c'))
        output = root/('debug output'+suffix)
        run(compiler,['-C',root,'build',source.name,'--release','--debug','--output',output])
        result = subprocess.run([str(output)],capture_output=True,text=True)
        assert result.returncode==0 and result.stdout=='42\n'
        source.write_text(explicit_entry('use "std/args"\nfn main() { print(args.get(0)) }', path=source))
        for value in ('--help','--version','--timings','--cwd','--engine'):
            run(runtime,[source,'--',value],value+'\n',env=empty_path)
        result = run(runtime,[source,'--timings','--','--timings'],'--timings\n',env=empty_path)
        assert 'runtime: load ' in result.stderr
    print(f'PASS: {count} CLI scenarios; all aliases, isolated tools, stdin, arguments, output paths and exit status')

if __name__=='__main__':
    main()
