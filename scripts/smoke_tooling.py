"""Exercise project resolution, formatter invariants, real stdio LSP and native debug symbols."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir',required=True)
    parser.add_argument('--lldb',action='store_true',help='require a working LLDB and exercise a real .dev breakpoint')
    args=parser.parse_args()
    binary=Path(args.bin_dir).resolve()/('d.exe' if os.name=='nt' else 'd')
    count=0
    with tempfile.TemporaryDirectory(prefix='dev-tooling-') as temporary:
        root=Path(temporary)
        def command(*args,cwd=root,error=None,env=None):
            nonlocal count
            result=subprocess.run([str(binary),*map(str,args)],cwd=cwd,env=env,capture_output=True,text=True,encoding='utf-8',timeout=45)
            if error: assert result.returncode!=0 and error in result.stderr,(args,result.stdout,result.stderr)
            else: assert result.returncode==0,(args,result.stdout,result.stderr)
            count+=1
            return result
        command('new','app'); command('new','library'); command('new','base')
        app=root/'app'; library=root/'library'; base=root/'base'
        (base/'src/lib.dev').write_text('fn number() i64 { return 42 }\n',encoding='utf-8')
        (library/'src/lib.dev').write_text('use "base/lib"\nfn answer() i64 { return lib.number() }\n',encoding='utf-8')
        command('pkg','add','base','--path',base,cwd=library)
        command('pkg','add','math','--path',library,cwd=app)
        (app/'src/main.dev').write_text('use "math/lib"\nprint(lib.answer())\n',encoding='utf-8')
        no_cc=dict(os.environ,DEV_CC='missing-compiler',PATH='')
        for engine in ('auto','ast'):
            assert command('run','--engine',engine,cwd=app,env=no_cc).stdout=='42\n'
        command('pkg','install','--locked',cwd=app)
        command('check',cwd=app)
        command('build','--release','-o',root/'native.exe',cwd=app)
        assert subprocess.check_output([str(root/'native.exe')],text=True)=='42\n'
        assert 'math' in command('pkg','list',cwd=app).stdout
        lock=tomllib.loads((app/'dev.lock').read_text())
        assert set(lock['packages'])=={'math','base'}
        (base/'src/lib.dev').write_text('fn number() i64 { return 43 }\n',encoding='utf-8')
        command('run',cwd=app,error='content changed')
        command('pkg','install',cwd=app)
        assert command('run',cwd=app).stdout=='43\n'
        command('pkg','remove','math',cwd=app)
        assert not tomllib.loads((app/'dev.lock').read_text())['packages']
        # Local Git repository: no external network, HEAD pinning and explicit update.
        def git(*args):
            return subprocess.check_output(['git','-C',str(base),*args],text=True).strip()
        git('init'); git('config','user.name','Tooling Test'); git('config','user.email','tooling@example.invalid')
        git('add','.'); git('commit','-m','initial'); first=git('rev-parse','HEAD')
        command('pkg','add','base','--git',base.as_uri(),cwd=app)
        (app/'src/main.dev').write_text('use "base/lib"\nprint(lib.number())\n',encoding='utf-8')
        assert command('run',cwd=app).stdout=='43\n'
        (base/'src/lib.dev').write_text('fn number() i64 { return 44 }\n',encoding='utf-8')
        git('add','.'); git('commit','-m','next')
        command('pkg','install',cwd=app)
        assert tomllib.loads((app/'dev.lock').read_text())['packages']['base']['commit']==first
        assert command('run',cwd=app).stdout=='43\n'
        first_lock=tomllib.loads((app/'dev.lock').read_text())['packages']['base']
        (app/first_lock['root']).rename(root/'held-checkout')
        old_lock=(app/'dev.lock').read_bytes()
        command('pkg','install','--locked',cwd=app)
        assert (app/'dev.lock').read_bytes()==old_lock
        assert command('run',cwd=app).stdout=='43\n'
        command('pkg','update',cwd=app)
        assert command('run',cwd=app).stdout=='44\n'
        locked=tomllib.loads((app/'dev.lock').read_text())['packages']['base']
        checkout=app/locked['root']
        (checkout/'src/lib.dev').write_text('fn number() i64 { return 99 }\n')
        command('pkg','install','--locked',cwd=app,error='dependency changed')
        command('run',cwd=app,error='content changed')
        command('pkg','add','std','--path',base,cwd=app,error='invalid dependency')
        command('pkg','add','bad','--git','http://example.invalid/a',cwd=app,error='https://')
        # Formatter: comments, object braces, escaped strings, CRLF, idempotence.
        code='fn main() {\r\nprint("{literal}") // " { comment\r\nlet x={\r\nname:"Dev",\r\n}\r\nprint(x.name)\r\n}\r\nmain()\r\n'
        sample=root/'format.dev'; sample.write_bytes(code.encode())
        command('fmt','--check',sample,error='needs formatting')
        expected='fn main() {\n    print("{literal}") // " { comment\n    let x={\n        name:"Dev",\n    }\n    print(x.name)\n}\nmain()\n'
        assert command('fmt','--stdout',sample).stdout==expected
        command('fmt',sample); command('fmt','--check',sample)
        assert command('run',sample,env=no_cc).stdout=='{literal}\nDev\n'
        multiline=root/'multiline.dev'; multiline.write_text('fn main() {\nlet s="one\\n  two  \\nthree"\nprint(s)\n}\nmain()\n',encoding='utf-8')
        before=command('run',multiline,env=no_cc).stdout
        command('fmt',multiline); command('fmt','--check',multiline)
        assert command('run',multiline,env=no_cc).stdout==before
        malformed=root/'bad.dev'; malformed.write_text('let x = {\n',encoding='utf-8')
        command('fmt',malformed,error='expected')
        assert malformed.read_text()=='let x = {\n'
        no_newline=root/'no-newline.dev'; no_newline.write_text('print(42)',encoding='utf-8')
        command('fmt',no_newline); command('fmt','--check',no_newline)
        # Standard LSP framing through an actual subprocess; responses + notifications.
        uri=sample.as_uri()
        valid='fn answer() i64 { return 42 }\nprint(answer())\n'
        messages=[
            {'id':0,'method':'textDocument/completion','params':{}},
            {'id':1,'method':'initialize','params':{'capabilities':{}}},
            {'method':'initialized','params':{}},
            {'method':'textDocument/didOpen','params':{'textDocument':{'uri':uri,'languageId':'devlang','version':1,'text':'let x ='}}},
            {'method':'textDocument/didChange','params':{'textDocument':{'uri':uri,'version':2},'contentChanges':[{'text':valid}]}},
            {'id':2,'method':'textDocument/completion','params':{'textDocument':{'uri':uri},'position':{'line':1,'character':6}}},
            {'id':3,'method':'textDocument/hover','params':{'textDocument':{'uri':uri},'position':{'line':1,'character':8}}},
            {'id':4,'method':'textDocument/definition','params':{'textDocument':{'uri':uri},'position':{'line':1,'character':8}}},
            {'id':5,'method':'textDocument/documentSymbol','params':{'textDocument':{'uri':uri}}},
            {'id':6,'method':'textDocument/formatting','params':{'textDocument':{'uri':uri},'options':{'tabSize':4,'insertSpaces':True}}},
            {'method':'textDocument/didChange','params':{'textDocument':{'uri':uri,'version':3},'contentChanges':[{'text':'print("😀"); let bad ='}]}},
            {'id':7,'method':'unknown/request','params':{}},
            {'method':'textDocument/didClose','params':{'textDocument':{'uri':uri}}},
            {'id':8,'method':'shutdown','params':None},
            {'method':'exit'},
        ]
        wire=b''
        for message in messages:
            body=json.dumps(dict(jsonrpc='2.0',**message),ensure_ascii=False).encode()
            wire+=f'Content-Length: {len(body)}\r\n\r\n'.encode()+body
        result=subprocess.run([str(binary),'lsp','--stdio'],input=wire,capture_output=True,timeout=10)
        assert result.returncode==0,result.stderr
        output=result.stdout; responses=[]
        while output:
            header,output=output.split(b'\r\n\r\n',1)
            length=int(header.split(b':')[1]); responses.append(json.loads(output[:length])); output=output[length:]
        replies={r['id']:r for r in responses if 'id' in r}
        assert replies[0]['error']['code']==-32002
        assert replies[1]['result']['capabilities']['textDocumentSync']['change']==1
        assert any(i['label']=='answer' for i in replies[2]['result']['items'])
        assert 'i64' in replies[3]['result']['contents']['value']
        assert replies[4]['result']['uri']==uri
        assert replies[5]['result'][0]['name']=='answer'
        assert replies[6]['result']==[]
        assert replies[7]['error']['code']==-32601
        diagnostics=[r['params'] for r in responses if r.get('method')=='textDocument/publishDiagnostics']
        assert diagnostics[0]['diagnostics'] and not diagnostics[1]['diagnostics']
        assert diagnostics[2]['diagnostics'][0]['range']['start']['character']==22,diagnostics[2]
        assert not diagnostics[-1]['diagnostics']
        count+=len(messages)
        # Debugger configuration must not overwrite existing editor files.
        command('debug','--vscode')
        config=json.loads((root/'.vscode/launch.json').read_text())
        assert config['configurations'][0]['type']=='lldb'
        command('debug','--vscode',error='already exists')
        debug_source=root/'debug.dev'
        debug_source.write_text('fn main() {\n    let number = 40\n    number += 2\n    print(number)\n}\nmain()\n',encoding='utf-8')
        command('debug',debug_source,'--no-launch')
        executable=root/'out/debug'/('app.exe' if os.name=='nt' else 'app')
        assert subprocess.check_output([str(executable)],text=True)=='42\n'
        if shutil.which('llvm-dwarfdump'):
            dwarf=subprocess.check_output(['llvm-dwarfdump','--debug-line',str(executable)],text=True)
            assert 'debug.dev' in dwarf and '0x' in dwarf,dwarf
            count+=1
        if args.lldb:
            result=subprocess.run([str(binary),'debug',str(debug_source),'--debugger','lldb','--batch','--command','breakpoint set --file debug.dev --line 4','--command','run','--command','frame variable','--command','continue'],cwd=root,capture_output=True,text=True,timeout=30)
            assert result.returncode==0 and 'stop reason = breakpoint' in result.stdout and '= 42' in result.stdout,(result.stdout,result.stderr)
            count+=1
        print(f'PASS: {count} tooling checks (project/path/Git lock, formatter, LSP, native symbols)')

if __name__=='__main__': main()
