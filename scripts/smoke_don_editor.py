"""Exercise DON unsaved-document LSP and lossless CLI formatting."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile


def session(binary, messages):
    sequence = [dict(id=1,method='initialize',params={}), *messages,
                dict(id=999,method='shutdown'), dict(method='exit')]
    wire = b''
    for message in sequence:
        body = json.dumps(dict(jsonrpc='2.0',**message),ensure_ascii=False).encode()
        wire += f'Content-Length: {len(body)}\r\n\r\n'.encode()+body
    result = subprocess.run([binary,'lsp','--stdio'],input=wire,capture_output=True,timeout=30)
    assert result.returncode == 0,result.stderr
    replies, remaining = [], result.stdout
    while remaining:
        header, remaining = remaining.split(b'\r\n\r\n',1)
        length = int(header.split(b':')[1])
        replies.append(json.loads(remaining[:length])); remaining = remaining[length:]
    return replies


def main():
    p = argparse.ArgumentParser(description=__doc__);p.add_argument('--bin',required=True)
    binary = str(Path(p.parse_args().bin).resolve())
    count = 0
    with tempfile.TemporaryDirectory(prefix='don-editor-') as work:
        file = Path(work)/'config.don';uri=file.as_uri()
        disk = '# disk unchanged\nname: "disk"\n';file.write_text(disk,encoding='utf-8')
        def opened(text, target=uri, language='don'):
            return dict(method='textDocument/didOpen',params=dict(textDocument=dict(uri=target,languageId=language,version=1,text=text)))
        def request(id,method,position=None,target=uri):
            params=dict(textDocument=dict(uri=target))
            if position is not None:params['position']=position
            return dict(id=id,method='textDocument/'+method,params=params)
        text="// keep\nversion='v1.0.0'\nserver:{host:'localhost';port:8080}\nrev:@version\nraw:\"\"\"Hello\n  Dev\"\"\"\n"
        replies=session(binary,[opened(text),request(10,'documentSymbol'),request(11,'formatting'),
                                request(12,'hover',dict(line=3,character=5)),request(13,'definition',dict(line=3,character=5))])
        byid={r['id']:r for r in replies if 'id' in r}
        diagnostics=[r['params'] for r in replies if r.get('method')=='textDocument/publishDiagnostics']
        assert diagnostics[0]['diagnostics']==[],diagnostics
        assert [s['name'] for s in byid[10]['result']]==['version','server','rev','raw']
        server=byid[10]['result'][1]
        assert [s['name'] for s in server['children']]==['host','port']
        formatted=byid[11]['result'][0]['newText']
        assert '// keep' in formatted and 'rev: @version' in formatted and '"""Hello\n  Dev"""' in formatted
        assert '@version: string' in byid[12]['result']['contents']['value']
        assert byid[13]['result']['uri']==uri
        assert byid[13]['result']['range']['start']==dict(line=1,character=0)
        assert file.read_text(encoding='utf-8')==disk
        count += 7
        file.write_text(text,encoding='utf-8')
        cli=subprocess.run([binary,'don','fmt-source',str(file)],capture_output=True,text=True,encoding='utf-8',check=True)
        assert cli.stdout==formatted
        file.write_text(formatted,encoding='utf-8')
        second=subprocess.run([binary,'don','fmt-source',str(file)],capture_output=True,text=True,encoding='utf-8',check=True)
        assert second.stdout==formatted
        count += 2
        for bad,part in [("a:1\na:2",'duplicate object key'),('a:@missing','missing reference target'),
                         ('a:@b\nb:@a','cyclic reference'),('a:[1,2','expected'),('a:"broken','unterminated'),
                         ("a:'\\q'",'invalid string escape'),('a:1 b:2','expected comma' )]:
            replies=session(binary,[opened(bad),dict(method='textDocument/didChange',params=dict(textDocument=dict(uri=uri,version=2),contentChanges=[dict(text='a: 1')])),
                                    dict(method='textDocument/didClose',params=dict(textDocument=dict(uri=uri)))])
            diagnostics=[r['params'] for r in replies if r.get('method')=='textDocument/publishDiagnostics']
            err=diagnostics[0]['diagnostics'][0]
            assert err['source']=='DON' and err['severity']==1 and part in err['message'],err
            assert diagnostics[1]['version']==2 and diagnostics[1]['diagnostics']==[]
            assert diagnostics[2]['diagnostics']==[]
            count += 3
        completion="version: 'v1'\nrev: @ve"
        replies=session(binary,[opened(completion),request(20,'completion',dict(line=1,character=8))])
        items=next(r['result']['items'] for r in replies if r.get('id')==20)
        item=next(i for i in items if i['label']=='@version')
        assert item['textEdit']['newText']=='version'
        assert item['textEdit']['range']['start']==dict(line=1,character=6)
        count += 1
        # Untitled documents are selected by languageId rather than extension.
        replies=session(binary,[opened('x: true',target='untitled:Untitled-1'),request(21,'documentSymbol',target='untitled:Untitled-1')])
        assert next(r['result'] for r in replies if r.get('id')==21)[0]['name']=='x'
        count += 1
        # Parser columns count scalars; LSP must report UTF-16 and tolerate CRLF/CR.
        for newline in ['\n','\r\n','\r']:
            text='emoji: "🚀"'+newline+'bad: @missing'
            replies=session(binary,[opened(text)])
            err=next(r['params']['diagnostics'][0] for r in replies if r.get('method')=='textDocument/publishDiagnostics')
            assert err['range']['start']==dict(line=1,character=5),err
            count += 1
        replies=session(binary,[opened('emoji: "🚀"; bad: @missing')])
        err=next(r['params']['diagnostics'][0] for r in replies if r.get('method')=='textDocument/publishDiagnostics')
        assert err['range']['start']['character']==18,err
        count += 1
    print(f'PASS: {count} DON editor checks; unsaved diagnostics, reference navigation/completion, outline, UTF-16 and formatting')


if __name__=='__main__':main()
