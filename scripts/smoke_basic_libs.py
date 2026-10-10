"""Run foundational library contracts in both source engines without a compiler."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir',required=True)
    args=parser.parse_args()
    d=Path(args.bin_dir).resolve()/('d.exe' if os.name=='nt' else 'd')
    count=0
    with tempfile.TemporaryDirectory(prefix='dev-basics-') as temporary:
        root=Path(temporary); source=root/'main.dev'
        env=dict(os.environ,PATH='',DEV_CC='missing-compiler')
        def run(code,expected=None,error=None,exit_code=0):
            nonlocal count
            source.write_text(code,encoding='utf-8')
            results=[]
            for engine in ('auto','ast'):
                r=subprocess.run([str(d),str(source),'--engine',engine],cwd=root,env=env,capture_output=True,text=True,encoding='utf-8',timeout=30)
                if error:
                    assert r.returncode != 0 and error in r.stderr,(code,r.stdout,r.stderr)
                    assert 'main.dev:' in r.stderr,r.stderr
                else:
                    assert r.returncode==exit_code,(code,r.stdout,r.stderr)
                    if expected is not None: assert r.stdout==expected,(code,r.stdout,r.stderr)
                results.append(r); count+=1
            return results
        run('''use "std/math"
use "std/test"
test.near(math.sqrt(9.0),3.0,0.000001,"sqrt")
test.near(math.cbrt(-8.0),-2.0,0.000001,"cbrt")
test.near(math.pow(2.0,10.0),1024.0,0.000001,"pow")
test.near(math.exp(1.0),math.e(),0.000001,"exp")
test.near(math.log(math.e()),1.0,0.000001,"ln")
test.near(math.log2(8.0),3.0,0.000001,"log2")
test.near(math.log10(100.0),2.0,0.000001,"log10")
test.near(math.sin(math.pi()/2.0),1.0,0.000001,"sin")
test.near(math.cos(0.0),1.0,0.000001,"cos")
test.near(math.tan(0.0),0.0,0.000001,"tan")
test.near(math.asin(1.0),math.pi()/2.0,0.000001,"asin")
test.near(math.acos(1.0),0.0,0.000001,"acos")
test.near(math.atan(1.0),math.pi()/4.0,0.000001,"atan")
test.near(math.atan2(1.0,0.0),math.pi()/2.0,0.000001,"atan2")
test.near(math.hypot(3.0,4.0),5.0,0.000001,"hypot")
test.near(math.tau(),2.0*math.pi(),0.000001,"tau")
test.equal(math.abs(-2.0),2.0,"abs")
test.equal(math.floor(-1.2),-2.0,"floor")
test.equal(math.ceil(-1.2),-1.0,"ceil")
test.equal(math.round(-1.5),-2.0,"round")
test.equal(math.trunc(-1.9),-1.0,"trunc")
test.equal(math.min(1.0,2.0),1.0,"min")
test.equal(math.max(1.0,2.0),2.0,"max")
test.equal(math.clamp(9.0,1.0,5.0),5.0,"clamp")
test.expect(math.isFinite(1.0),"finite")
test.expect(not math.isNaN(1.0),"nan")
print("math OK")
''','math OK\n')
        for code,error in [('math.sqrt(-1.0)','domain'),('math.log(0.0)','domain'),('math.exp(1000.0)','overflow'),('math.clamp(1.0,2.0,0.0)','minimum'),('math.asin(2.0)','domain')]:
            run('use "std/math"\n'+code+'\n',error=error)
        run('''use "std/random"
use "std/test"
use "std/buffer"
random.seed(0)
test.near(random.float(),0.8833108082136426,0.000000000000001,"splitmix64")
random.seed(0)
let data = buffer.fromBytes(random.bytes(16))
test.equal(data.toString("hex"),"afcd1d7b39a820e2f465b9a16a9e786e","seeded bytes")
data.close()
random.seed(-42)
let first = random.int(-100,100)
random.seed(-42)
test.equal(random.int(-100,100),first,"repeat seed")
for i in 0..1000 {
    let n=random.int(-7,11)
    test.expect(n >= -7 && n < 11,"bounds")
    let f=random.float()
    test.expect(f >= 0.0 && f < 1.0,"float bounds")
}
test.equal(random.int(4,5),4,"singleton")
test.expect(random.bytes(0).len() == 0 as usize,"empty bytes")
let flag=random.bool()
test.expect(flag || not flag,"bool")
let wide=random.int(-9223372036854775808,9223372036854775807)
test.expect(wide < 9223372036854775807,"wide range")
print("random OK")
''','random OK\n')
        run('use "std/random"\nlet f=random.float()\nprint(f >= 0.0 && f < 1.0)\n','true\n')
        for code,error in [('random.int(1,1)','min < max'),('random.int(5,1)','min < max'),('random.bytes(-1)','negative'),('random.bytes(8388609)','exceeds')]:
            run('use "std/random"\n'+code+'\n',error=error)
        run('''use "std/strings"
use "std/test"
test.equal(strings.len("Aé🚀"),7 as usize,"bytes")
test.equal(strings.charLength("Aé🚀"),3,"characters")
test.equal(strings.charAt("Aé🚀",1),"é","char")
test.equal(strings.substring("Aé🚀",1,3),"é🚀","substring")
test.equal(strings.substring("hello",5,5),"","empty")
test.equal(strings.indexOf("Aé🚀","🚀"),3,"byte index")
test.equal(strings.lastIndexOf("x-x-x","x"),4,"last index")
test.equal(strings.indexOf("abc","z"),-1,"missing")
test.equal(strings.trim("  é  "),"é","trim")
test.equal(strings.trimStart("  é  "),"é  ","trim start")
test.equal(strings.trimEnd("  é  "),"  é","trim end")
test.equal(strings.toUpperCase("straße"),"STRASSE","upper")
test.equal(strings.toLowerCase("DEV"),"dev","lower")
test.expect(strings.contains("abc","b"),"contains")
test.expect(strings.startsWith("abc","a"),"starts")
test.expect(strings.endsWith("abc","c"),"ends")
test.expect(strings.equal(strings.concat("a","b"),"ab"),"old APIs")
test.equal(strings.repeat("é",3),"ééé","repeat")
test.equal(strings.replace("aaa","a","b"),"baa","replace")
test.equal(strings.replaceAll("aaa","a","b"),"bbb","replace all")
test.equal(strings.replaceAll("ab","","-"),"-a-b-","empty pattern")
test.equal(strings.join(strings.split("a,,b,",","),"|"),"a||b|","split empties")
test.equal(strings.join(strings.split("é🚀",""),"|"),"é|🚀","unicode split")
test.equal(strings.split("","").len(),0 as usize,"empty split")
print("strings OK")
''','strings OK\n')
        for code,error in [('strings.substring("é",0,2)','bounds'),('strings.charAt("é",1)','bounds'),('strings.repeat("x",-1)','negative'),('strings.repeat("xx",8388608)','exceeds'),('strings.split(strings.repeat("x",65537),"")','65536')]:
            run('use "std/strings"\n'+code+'\n',error=error)
        run('''use "std/datetime"
use "std/test"
let stamp=datetime.utc(2024,2,29,12,30,0,123)
test.equal(datetime.iso(stamp),"2024-02-29T12:30:00.123Z","iso")
test.equal(datetime.parse("2024-02-29T19:30:00.123+07:00"),stamp,"offset parse")
test.equal(datetime.format(stamp,"%F %T"),"2024-02-29 12:30:00","format")
test.equal(datetime.formatOffset(stamp,420,"%F %T %:z"),"2024-02-29 19:30:00 +07:00","offset")
let parts=datetime.parts(stamp,420)
test.equal(parts.year,2024,"year")
test.equal(parts.month,2,"month")
test.equal(parts.day,29,"day")
test.equal(parts.hour,19,"hour")
test.equal(parts.minute,30,"minute")
test.equal(parts.second,0,"second")
test.equal(parts.millisecond,123,"millisecond")
test.equal(parts.weekday,4,"weekday")
test.equal(parts.offsetMinutes,420,"offset minutes")
test.equal(datetime.iso(-1),"1969-12-31T23:59:59.999Z","negative epoch")
test.equal(datetime.addMilliseconds(0,-1),-1,"addition")
test.expect(datetime.isLeapYear(2000) && not datetime.isLeapYear(1900),"leap rules")
test.equal(datetime.daysInMonth(2024,2),29,"february")
test.equal(datetime.daysInMonth(2023,2),28,"february normal")
test.equal(datetime.daysInMonth(2024,4),30,"april")
test.expect(datetime.now()>1700000000000,"system clock")
print("datetime OK")
''','datetime OK\n')
        for code,error in [('datetime.utc(2023,2,29,0,0,0,0)','invalid'),('datetime.parse("2024-01-01")','RFC3339'),('datetime.parse("2016-12-31T23:59:60Z")','leap'),('datetime.parts(0,1440)','offset'),('datetime.format(0,"%Q")','directive'),('datetime.format(0,"%#z")','directive'),('datetime.daysInMonth(2024,13)','month'),('datetime.iso(9223372036854775807)','calendar'),('datetime.parts(datetime.utc(262142,12,31,23,59,59,999),1439)','calendar')]:
            run('use "std/datetime"\n'+code+'\n',error=error)
        run('''use "std/test"
test.case("pass",fn() { test.equal("a","a","text") })
test.case("fail",fn() { test.expect(false,"expected failure") })
test.case("after",fn() { test.near(1.0,1.001,0.01,"near") })
let report=test.run()
print(report.total)
print(report.passed)
print(report.failed)
print(test.run().total)
''','3\n2\n1\n0\n')
        run('''use "std/test"
use "std/json"
test.equal({name:"Dev",items:[1,2]},{items:[1,2],name:"Dev"},"objects")
let items=Vec<i64>()
items.push(3)
test.equal(items,items,"vectors")
print("equal OK")
''','equal OK\n')
        run('''use "std/test"
fn main() {
    test.case("failure",fn() { test.expect(false,"failure") })
    let result=test.run()
    return result.failed as i32
}
return main()
''',exit_code=1)
        for code,error in [('test.expect(false,"bad")','assertion failed'),('test.near(1.0,1.0,-1.0,"bad")','nonnegative'),('test.case("same",fn() {}); test.case("same",fn() {})','duplicate')]:
            run('use "std/test"\n'+code+'\n',error=error)
        run('''use "std/test"
test.case("nested",fn() { test.run() })
let report=test.run()
print(report.failed)
test.case("again",fn() { test.expect(true,"recovered") })
print(test.run().passed)
''','1\n1\n')
        results=run('''use "std/log"
log.debug("hidden")
log.info("ready\\nnext")
log.warn("warning")
log.setLevel("off")
log.error("hidden too")
''','')
        for result in results:
            assert '[INFO] ready\\nnext' in result.stderr and '[WARN] warning' in result.stderr,result.stderr
            assert 'hidden' not in result.stderr and len(result.stderr.splitlines())==2,result.stderr
        run('''use "std/log"
use "std/fs"
use "std/strings"
fs.write_text("test.log","")
log.toFile("test.log")
log.setLevel("warn")
log.info("hidden")
log.warn("visible")
log.error("problem")
log.flush()
log.toStderr()
let text=fs.read_text("test.log")
print(strings.contains(text,"[WARN] visible"))
print(strings.contains(text,"[ERROR] problem"))
print(not strings.contains(text,"hidden"))
fs.remove_file("test.log")
''','true\ntrue\ntrue\n')
        run('use "std/log"\nlog.setLevel("verbose")\n',error='log level')
        run('use "std/log"\nlog.toFile("missing/folder/log.txt")\n',error='std/log.toFile')
    print(f'PASS: {count} basic-library executions; math/random/strings/datetime/test/log, auto/AST')

if __name__=='__main__': main()
