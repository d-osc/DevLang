"""Exercise the real LSP workspace index and unsaved import navigation."""
import json,subprocess,tempfile,argparse
from pathlib import Path

def request(id,method,params):return dict(id=id,method=method,params=params)
def opened(uri,text,version=1):return dict(method='textDocument/didOpen',params={'textDocument':{'uri':uri,'languageId':'devlang','version':version,'text':text}})
def send(binary,messages):
 data=b''
 for m in messages:
  body=json.dumps(dict(jsonrpc='2.0',**m)).encode();data+=f'Content-Length: {len(body)}\r\n\r\n'.encode()+body
 r=subprocess.run([str(binary),'lsp','--stdio'],input=data,capture_output=True,timeout=30);assert r.returncode==0,r.stderr
 rows={};data=r.stdout
 while data:
  header,data=data.split(b'\r\n\r\n',1);length=int(header.split(b':')[1]);row=json.loads(data[:length]);data=data[length:]
  if 'id' in row:assert 'error' not in row,row;rows[row['id']]=row['result']
 return rows

def main():
 args=argparse.ArgumentParser();args.add_argument('--bin',required=True);binary=Path(args.parse_args().bin).resolve()
 with tempfile.TemporaryDirectory() as d:
  root=Path(d);helper=root/'helper.dev';helper.write_text('fn add(a:i64,b:i64):i64{return a+b}\n',encoding='utf-8')
  main=root/'main.dev';text='use "./helper" as math\nfn main(){print(math.add(1,2));print(Math.sin(0))}\nmain()\n';main.write_text(text,encoding='utf-8');uri=main.as_uri();helper_uri=helper.as_uri()
  base={'textDocument':{'uri':uri}};add=text.splitlines()[1].index('add')+1;sin=text.splitlines()[1].index('sin')+1
  (root/'target').mkdir();(root/'target/ignored.dev').write_text('fn should_not_index(){}',encoding='utf-8')
  messages=[request(1,'initialize',{'rootUri':root.as_uri()}),opened(uri,text),request(2,'workspace/symbol',{'query':'add'}),request(3,'textDocument/definition',dict(base,position={'line':1,'character':add})),request(4,'textDocument/hover',dict(base,position={'line':1,'character':add})),request(5,'textDocument/completion',dict(base,position={'line':1,'character':add})),request(6,'textDocument/completion',dict(base,position={'line':1,'character':sin})),opened(helper_uri,'fn add(a:f64,b:f64):f64{return a+b}\n',2),request(7,'textDocument/hover',dict(base,position={'line':1,'character':add})),request(8,'workspace/symbol',{'query':'should_not_index'}),dict(method='textDocument/didClose',params={'textDocument':{'uri':helper_uri}}),request(9,'textDocument/hover',dict(base,position={'line':1,'character':add})),request(10,'shutdown',{}),dict(method='exit')]
  rows=send(binary,messages);assert rows[1]['capabilities']['workspaceSymbolProvider'];assert any(x['location']['uri']==helper_uri for x in rows[2]);assert rows[3]['uri']==helper_uri,rows[3];assert 'i64' in rows[4]['contents']['value'];assert any(x['label']=='add' for x in rows[5]['items']);assert any(x['label']=='f16round' for x in rows[6]['items']);assert 'f64' in rows[7]['contents']['value'];assert rows[8]==[];assert 'i64' in rows[9]['contents']['value'];assert helper.read_text()=='fn add(a:i64,b:i64):i64{return a+b}\n'
 print('PASS workspace symbols, import completion/definition/hover, dirty overlays, close refresh and exclusions')
if __name__=='__main__':main()
