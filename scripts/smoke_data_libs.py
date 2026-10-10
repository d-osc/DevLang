"""Cross-check runtime data libraries against Python codecs and Node AES-GCM."""
import argparse
import gzip
import hashlib
import hmac
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import zipfile
import zlib

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir',required=True)
    args=parser.parse_args()
    d=Path(args.bin_dir).resolve()/('d.exe' if os.name=='nt' else 'd')
    node=shutil.which('node')
    count=0
    with tempfile.TemporaryDirectory(prefix='dev-data-') as temporary:
        root=Path(temporary); source=root/'main.dev'
        env=dict(os.environ,PATH='',DEV_CC='missing-compiler')
        def run(code,expected=None,error=None):
            nonlocal count
            source.write_text(code,encoding='utf-8'); results=[]
            for engine in ('auto','ast'):
                r=subprocess.run([str(d),str(source),'--engine',engine],cwd=root,env=env,capture_output=True,text=True,encoding='utf-8',timeout=30)
                if error:
                    assert r.returncode!=0 and error in r.stderr,(code,r.stdout,r.stderr)
                    assert 'main.dev:' in r.stderr,r.stderr
                else:
                    assert r.returncode==0,(code,r.stdout,r.stderr)
                    if expected is not None: assert r.stdout==expected,(code,r.stdout,r.stderr)
                results.append(r); count+=1
            return results
        run('''use "std/regex"
use "std/test"
use "std/strings"
let r=regex.compile("(?<word>[a-z]+)=([0-9]+)","i")
test.expect(r.test("PORT=3000"),"case insensitive")
let found=r.find("é PORT=3000!")
test.expect(found.matched,"found")
test.equal(found.text,"PORT=3000","text")
test.equal(found.start,3,"byte offset")
test.equal(found.end,12,"byte end")
test.equal(r.find("none").start,-1,"missing")
test.equal(strings.join(r.findAll("a=1 b=2"),"|"),"a=1|b=2","all")
test.equal(strings.join(r.captures("x=42"),"|"),"x=42|x|42","captures")
test.equal(r.captures("none").len(),0 as usize,"no capture")
test.equal(r.replace("a=1 b=2","${word}:$2"),"a:1 b=2","replace")
test.equal(r.replaceAll("a=1 b=2","$$${1}-$2"),"$a-1 $b-2","replace all")
r.close()
let separator=regex.compile("[,;]","")
test.equal(strings.join(separator.split("a,b;c"),"|"),"a|b|c","split")
separator.close()
let literal=regex.compile(regex.escape("a+b"),"")
test.expect(literal.test("a+b"),"escape")
literal.close()
let optional=regex.compile("(a)?b","")
test.equal(optional.captures("b")[1],"","missing optional")
optional.close()
let multiline=regex.compile("^x.$","ms")
test.expect(multiline.test("x\\n"),"flags")
multiline.close()
let unicode=regex.compile("é+","")
test.equal(unicode.find("éé").end,4,"UTF8")
unicode.close()
let empty=regex.compile("","")
test.equal(empty.replaceAll("ab","-"),"-a-b-","empty matches")
empty.close()
print("regex OK")
''','regex OK\n')
        for code,error in [('regex.compile("[","")','invalid regex'),('regex.compile("x","g")','flags'),('regex.compile("x","ii")','duplicate'),('let r=regex.compile("x",""); r.close(); r.test("x")','closed'),('regex.compile("(?=x)","")','invalid regex')]:
            run('use "std/regex"\n'+code+'\n',error=error)
        run('use "std/regex"\nuse "std/strings"\nlet r=regex.compile("x","")\nr.findAll(strings.repeat("x",4097))\n',error='4096')
        run('use "std/regex"\nuse "std/strings"\nlet r=regex.compile("(.+)","")\nr.captures(strings.repeat("x",5000000))\n',error='8 MiB')
        run('use "std/regex"\nuse "std/strings"\nlet r=regex.compile("(.+)","")\nr.replace(strings.repeat("x",5000000),"$1$1")\n',error='8 MiB')
        run('use "std/regex"\nfor i in 0..33 { regex.compile("x","") }\n',error='32')
        run('''use "std/encoding"
use "std/test"
let text="DevLang é🚀"
test.equal(encoding.decode(encoding.encode(text,"utf-16le"),"utf-16le"),text,"LE")
test.equal(encoding.decode(encoding.encode(text,"utf-16be"),"utf-16be"),text,"BE")
test.equal(encoding.decode(encoding.transcode(encoding.encode(text,"utf-16le"),"utf-16le","utf-8"),"utf-8"),text,"transcode")
test.equal(encoding.decode(encoding.encode("€é","windows-1252"),"windows-1252"),"€é","legacy")
test.equal(encoding.decode(encoding.encode("日本語","shift_jis"),"shift_jis"),"日本語","shift jis")
test.expect(encoding.supported("utf8") && not encoding.supported("unknown"),"supported")
print("encoding OK")
''','encoding OK\n')
        fromhex='''use "std/buffer"
fn fromHex(text str) Vec<u8> {
    let b=buffer.from(text,"hex")
    let data=b.toBytes()
    b.close()
    return data
}
'''
        for code,error in [('encoding.decode(fromHex("ff"),"utf-8")','invalid byte'),('encoding.decode(fromHex("00"),"utf-16le")','even'),('encoding.decode(fromHex("00d8"),"utf-16le")','surrogate'),('encoding.encode("🚀","windows-1252")','representable'),('encoding.encode("a","unknown")','unsupported')]:
            run(fromhex+'use "std/encoding"\n'+code+'\n',error=error)
        for charset,python_charset in [('utf-16le','utf-16-le'),('utf-16be','utf-16-be'),('utf-8','utf-8'),('windows-1252','cp1252')]:
            text='Dev é€'
            expected=text.encode(python_charset).hex()
            run(fromhex+f'''use "std/encoding"
let bytes=encoding.encode({json.dumps(text,ensure_ascii=False)},{json.dumps(charset)})
let b=buffer.fromBytes(bytes)
print(b.toString("hex"))
b.close()
''',expected+'\n')
        for algorithm in ('sha256','sha512'):
            digest=getattr(hashlib,algorithm)(b'abc').hexdigest()
            tag=hmac.new(b'key',b'message',algorithm).hexdigest()
            run(fromhex+f'''use "std/crypto"
use "std/encoding"
use "std/test"
test.equal(crypto.hashText("{algorithm}","abc"),"{digest}","hash text")
test.equal(crypto.hash("{algorithm}",encoding.encode("abc","utf8")),fromHex("{digest}"),"hash bytes")
let key=encoding.encode("key","utf8")
let data=encoding.encode("message","utf8")
test.equal(crypto.hmac("{algorithm}",key,data),fromHex("{tag}"),"HMAC vector")
test.expect(crypto.verifyHmac("{algorithm}",key,data,fromHex("{tag}")),"verify")
test.expect(not crypto.verifyHmac("{algorithm}",key,data,fromHex("00")),"wrong tag")
print("crypto OK")
''','crypto OK\n')
        run('''use "std/crypto"
use "std/encoding"
use "std/test"
let a=crypto.secureBytes(32)
let b=crypto.secureBytes(32)
test.expect(not crypto.timingSafeEqual(a,b),"fresh entropy")
test.expect(crypto.timingSafeEqual(a,a),"equal")
test.expect(not crypto.timingSafeEqual(a,crypto.secureBytes(0)),"different lengths")
let data=encoding.encode("DevLang 🚀","utf8")
let aad=encoding.encode("context","utf8")
let packet=crypto.encrypt(a,data,aad)
test.equal(crypto.decrypt(a,packet,aad),data,"authenticated round trip")
test.equal(crypto.decrypt(a,crypto.encrypt(a,Vec<u8>(),Vec<u8>()),Vec<u8>()),Vec<u8>(),"empty")
print("AES OK")
''','AES OK\n')
        if node:
            fixture=json.loads(subprocess.check_output([node,'-e',"const c=require('node:crypto');const k=Buffer.alloc(32),n=Buffer.alloc(12),aad=Buffer.from('context'),p=Buffer.from('DevLang 🚀');const e=c.createCipheriv('aes-256-gcm',k,n);e.setAAD(aad);const b=Buffer.concat([n,e.update(p),e.final(),e.getAuthTag()]);console.log(JSON.stringify({key:k.toString('hex'),packet:b.toString('hex')}));"],text=True,encoding='utf-8'))
            results=run(fromhex+f'''use "std/crypto"
use "std/encoding"
use "std/test"
let key=fromHex("{fixture['key']}")
let aad=encoding.encode("context","utf8")
let text="DevLang 🚀"
test.equal(encoding.decode(crypto.decrypt(key,fromHex("{fixture['packet']}"),aad),"utf8"),text,"Node packet")
let encrypted=buffer.fromBytes(crypto.encrypt(key,encoding.encode(text,"utf8"),aad))
print(encrypted.toString("hex"))
encrypted.close()
''')
            for result in results:
                packet=result.stdout.strip()
                decoded=subprocess.check_output([node,'-e',"const c=require('node:crypto'),p=Buffer.from(process.argv[1],'hex');const d=c.createDecipheriv('aes-256-gcm',Buffer.alloc(32),p.subarray(0,12));d.setAAD(Buffer.from('context'));d.setAuthTag(p.subarray(-16));process.stdout.write(Buffer.concat([d.update(p.subarray(12,-16)),d.final()]));",packet],text=True,encoding='utf-8')
                assert decoded=='DevLang 🚀',decoded
            tampered=bytearray.fromhex(fixture['packet']);tampered[12]^=1
            run(fromhex+f'use "std/crypto"\nuse "std/encoding"\ncrypto.decrypt(fromHex("{fixture["key"]}"),fromHex("{tampered.hex()}"),encoding.encode("context","utf8"))\n',error='authentication')
        for code,error in [('crypto.hashText("md5","abc")','algorithm'),('crypto.secureBytes(-1)','negative'),('crypto.encrypt(Vec<u8>(),Vec<u8>(),Vec<u8>())','32 bytes'),('crypto.decrypt(crypto.secureBytes(32),Vec<u8>(),Vec<u8>())','nonce'),('let key=crypto.secureBytes(32); let p=crypto.encrypt(key,Vec<u8>(),Vec<u8>()); crypto.decrypt(crypto.secureBytes(32),p,Vec<u8>())','authentication'),('let key=crypto.secureBytes(32); let p=crypto.encrypt(key,Vec<u8>(),Vec<u8>()); let aad=Vec<u8>(); aad.push(1 as u8); crypto.decrypt(key,p,aad)','authentication')]:
            run('use "std/crypto"\n'+code+'\n',error=error)
        run('''use "std/compression"
use "std/encoding"
use "std/test"
let data=encoding.encode("DevLang é🚀","utf8")
test.equal(compression.gunzip(compression.gzip(data,6)),data,"gzip")
test.equal(compression.unzlib(compression.zlib(data,6)),data,"zlib")
test.equal(compression.inflate(compression.deflate(data,6)),data,"deflate")
test.equal(compression.gunzip(compression.gzip(Vec<u8>(),0)),Vec<u8>(),"empty")
print("compression OK")
''','compression OK\n')
        data=b'Hello DevLang\x00\xff'+b'x'*1000
        fixture_ops=[('gzip','gunzip',gzip.compress(data)),('zlib','unzlib',zlib.compress(data)),('deflate','inflate',zlib.compress(data,wbits=-15))]
        for encoder,decoder,encoded in fixture_ops:
            run(fromhex+f'''use "std/compression"
use "std/test"
let data=fromHex("{data.hex()}")
test.equal(compression.{decoder}(fromHex("{encoded.hex()}")),data,"Python fixture")
let b=buffer.fromBytes(compression.{encoder}(data,6))
print(b.toString("hex"))
b.close()
''')
            # Independent consumer of DevLang's compressed output.
            for result in run(fromhex+f'use "std/compression"\nlet b=buffer.fromBytes(compression.{encoder}(fromHex("{data.hex()}"),6))\nprint(b.toString("hex"))\nb.close()\n'):
                encoded_dev=bytes.fromhex(result.stdout.strip())
                decoded=gzip.decompress(encoded_dev) if encoder=='gzip' else zlib.decompress(encoded_dev,wbits=-15 if encoder=='deflate' else 15)
                assert decoded==data
            run(fromhex+f'use "std/compression"\ncompression.{decoder}(fromHex("{encoded[:-2].hex()}"))\n',error='std/compression')
            run(fromhex+f'use "std/compression"\ncompression.{decoder}(fromHex("{(encoded+b"junk").hex()}"))\n',error='std/compression')
        run('use "std/compression"\ncompression.gzip(Vec<u8>(),10)\n',error='level')
        bomb=gzip.compress(b'x'*(8*1024*1024+1))
        run(fromhex+f'use "std/compression"\ncompression.gunzip(fromHex("{bomb.hex()}"))\n',error='8 MiB')
        run('''use "std/archive"
use "std/encoding"
use "std/test"
let entries=Vec<archive.Entry>()
entries.push(archive.Entry("dir/hello.txt",encoding.encode("Dev é🚀","utf8")))
entries.push(archive.Entry("empty.txt",Vec<u8>()))
let zip=archive.writeZIP(entries)
let tar=archive.writeTAR(entries)
test.equal(archive.listZIP(zip)[0],"dir/hello.txt","zip list")
test.equal(archive.listTAR(tar)[0],"dir/hello.txt","tar list")
test.equal(encoding.decode(archive.readZIP(zip,"dir/hello.txt"),"utf8"),"Dev é🚀","zip data")
test.equal(encoding.decode(archive.readTAR(tar,"dir/hello.txt"),"utf8"),"Dev é🚀","tar data")
test.equal(archive.readZIP(zip,"empty.txt"),Vec<u8>(),"empty zip file")
test.equal(archive.readTAR(tar,"empty.txt"),Vec<u8>(),"empty tar file")
test.equal(archive.listZIP(archive.writeZIP(Vec<archive.Entry>())).len(),0 as usize,"empty zip")
test.equal(archive.listTAR(archive.writeTAR(Vec<archive.Entry>())).len(),0 as usize,"empty tar")
print("archive OK")
''','archive OK\n')
        for kind in ('ZIP','TAR'):
            result_code=f'''use "std/archive"
use "std/fs"
use "std/encoding"
let entries=Vec<archive.Entry>()
entries.push(archive.Entry("hello.txt",encoding.encode("hello","utf8")))
fs.write_bytes("created.{kind.lower()}",archive.write{kind}(entries))
'''
            run(result_code)
            if kind=='ZIP':
                with zipfile.ZipFile(root/'created.zip') as z: assert z.read('hello.txt')==b'hello'
                stream=io.BytesIO()
                with zipfile.ZipFile(stream,'w',compression=zipfile.ZIP_DEFLATED) as z: z.writestr('hello.txt',b'hello')
            else:
                with tarfile.open(root/'created.tar') as t: assert t.extractfile('hello.txt').read()==b'hello'
                stream=io.BytesIO()
                with tarfile.open(fileobj=stream,mode='w') as t:
                    info=tarfile.TarInfo('hello.txt'); info.size=5; t.addfile(info,io.BytesIO(b'hello'))
            fixture_path=root/f'python.{kind.lower()}'; fixture_path.write_bytes(stream.getvalue())
            run(f'use "std/archive"\nuse "std/fs"\nuse "std/encoding"\nprint(encoding.decode(archive.read{kind}(fs.read_bytes("{fixture_path.name}"),"hello.txt"),"utf8"))\n','hello\n')
            run(f'use "std/archive"\nuse "std/fs"\narchive.read{kind}(fs.read_bytes("{fixture_path.name}"),"missing")\n',error='not found')
            for name in ('../escape','/absolute','C:/drive','a\\b','a/../b'):
                run(f'use "std/archive"\nlet files=Vec<archive.Entry>()\nfiles.push(archive.Entry({json.dumps(name)},Vec<u8>()))\narchive.write{kind}(files)\n',error='relative')
            run(f'use "std/archive"\nlet files=Vec<archive.Entry>()\nfiles.push(archive.Entry("same",Vec<u8>()))\nfiles.push(archive.Entry("same",Vec<u8>()))\narchive.write{kind}(files)\n',error='duplicate')
        # External hostile fixtures: traversal names and symlinks must never reach disk.
        stream=io.BytesIO()
        with zipfile.ZipFile(stream,'w') as z:z.writestr('../escape',b'hello')
        (root/'unsafe.zip').write_bytes(stream.getvalue())
        run('use "std/archive"\nuse "std/fs"\narchive.listZIP(fs.read_bytes("unsafe.zip"))\n',error='relative')
        stream=io.BytesIO()
        with tarfile.open(fileobj=stream,mode='w') as t:
            info=tarfile.TarInfo('link'); info.type=tarfile.SYMTYPE;info.linkname='../escape';t.addfile(info)
        (root/'links.tar').write_bytes(stream.getvalue())
        run('use "std/archive"\nuse "std/fs"\narchive.listTAR(fs.read_bytes("links.tar"))\n',error='links')
        (root/'short.tar').write_bytes((root/'python.tar').read_bytes()[:512])
        run('use "std/archive"\nuse "std/fs"\narchive.listTAR(fs.read_bytes("short.tar"))\n',error='truncated')
        run('''use "std/uuid"
use "std/test"
test.equal(uuid.nil(),"00000000-0000-0000-0000-000000000000","nil")
test.equal(uuid.parse("550E8400-E29B-41D4-A716-446655440000"),"550e8400-e29b-41d4-a716-446655440000","canonical")
test.expect(uuid.isValid(uuid.v4()),"valid")
test.equal(uuid.version(uuid.v4()),4,"version")
test.expect(uuid.v4()!=uuid.v4(),"different")
test.expect(not uuid.isValid("not-a-uuid"),"invalid")
print("UUID OK")
''','UUID OK\n')
        run('use "std/uuid"\nuuid.parse("bad")\n',error='UUID')
    print(f'PASS: {count} data-library executions across auto/AST; AES Node interoperability: {bool(node)}')

if __name__=='__main__':main()
