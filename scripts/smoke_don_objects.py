"""DON-style data objects in Dev source and lossless editor formatting."""
import argparse,os,subprocess,tempfile
from pathlib import Path

def main():
 p=argparse.ArgumentParser();p.add_argument('--bin-dir',required=True);folder=Path(p.parse_args().bin_dir).resolve();suffix='.exe' if os.name=='nt' else ''
 code='let name: str = \'Dev\'\nlet user = {\nname = name\ngreeting: "Hello ${name}"\nsettings: { enabled = true; label: \'demo\' }\ntags: [\n\'dev\'\n\'lang\'\n]\n/* comment */\nnote: """line 1\nline 2"""\n}\nlet before=user\nuser.name=\'New\'\nprint(before.name)\nprint(user.name)\nprint(user.greeting)\nprint(user.tags[1])\nprint(user.settings.enabled)\nprint(user.note)\n'
 with tempfile.TemporaryDirectory() as directory:
  source=Path(directory)/'main.dev';source.write_text(code,encoding='utf-8',newline='\n')
  for mode in ['auto','ast']:
   result=subprocess.run([str(folder/('devrun'+suffix)),str(source),'--engine',mode],text=True,capture_output=True,encoding='utf-8',timeout=30)
   assert result.returncode==0 and result.stdout=='Dev\nNew\nHello Dev\nlang\ntrue\nline 1\nline 2\n',(mode,result.stdout,result.stderr)
  formatted=subprocess.check_output([str(folder/('d'+suffix)),'fmt','--stdout',str(source)],text=True,encoding='utf-8')
  source.write_text(formatted,encoding='utf-8',newline='\n')
  assert subprocess.check_output([str(folder/('d'+suffix)),'fmt','--stdout',str(source)],text=True,encoding='utf-8')==formatted
  for code in ["let x={a=1 a=2}","let x={a=1; a=2}"]:
   source.write_text(code)
   result=subprocess.run([str(folder/('devrun'+suffix)),str(source)],text=True,capture_output=True,timeout=30)
   assert result.returncode!=0 and ('between object fields' in result.stderr or 'duplicate object' in result.stderr),result.stderr
 print('PASS: DON objects in auto/AST, field access, mutation, copy, multiline text and formatter')

if __name__=='__main__':main()
