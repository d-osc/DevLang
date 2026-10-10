"""Build per-module references from shared signatures and reviewed API contracts.

Run --check in CI to detect stale references or missing module coverage. Optional
--d runs the documented examples in temporary working directories.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
CATALOG = ROOT / 'docs/modules/catalog.json'


def signatures(text):
    """Keep public declarations, excluding implementation/stub function bodies."""
    declarations = []
    for match in re.finditer(r'^(?:fn [^{\n]+|(?:struct|enum) [^{\n]+\{[^}]*\})', text, re.M):
        declaration = match.group().strip()
        if declaration.startswith('fn ') and declaration.endswith(')'):
            declaration += ' void'
        declarations.append(declaration)
    return '\n'.join(declarations)


def section(text, heading):
    start = re.search(r'^## ' + re.escape(heading) + r'\s*$', text, re.M)
    if not start:
        raise ValueError(f'Missing contract section: {heading}')
    following = re.search(r'^## ', text[start.end():], re.M)
    return text[start.start():start.end() + following.start() if following else len(text)].strip()


def links(text, source):
    def replace(match):
        label, target = match.groups()
        if re.match(r'^(?:https?:|#|mailto:)', target):
            return match.group()
        path, _, anchor = target.partition('#')
        relative = os.path.relpath((source.parent / path).resolve(), ROOT / 'docs/modules').replace('\\', '/')
        return f'[{label}]({relative}' + (f'#{anchor}' if anchor else '') + ')'
    return re.sub(r'\[([^\]]+)\]\(([^)]+)\)', replace, text)


def generate():
    catalog = json.loads(CATALOG.read_text(encoding='utf-8'))
    site_catalog = ROOT / 'website/content/module-docs.json'
    if json.loads(site_catalog.read_text(encoding='utf-8')) != catalog:
        raise ValueError('Website module catalog differs from docs/modules/catalog.json')
    sources = '\n'.join(p.read_text(encoding='utf-8') for p in sorted((ROOT/'syntax/src').glob('*intrinsics.rs')))
    constants = dict(re.findall(r'pub const (\w+): &str = r#"(.*?)"#;', sources, re.S))
    modules = dict(re.findall(r'"std/([\w/]+)" => Some\((\w+)\)', sources))
    runtime = {key for key, item in catalog.items() if item['mode'] == 'runtime'}
    if runtime != set(modules) | {'io', 'time', 'args'}:
        raise ValueError('Module catalog does not cover the runtime registry exactly')
    builtin_block = re.search(r'pub const BUILTINS:.*?= &\[(.*?)\];', sources, re.S).group(1)
    if {'std/' + key for key in runtime} != set(re.findall(r'"([^"]+)"', builtin_block)):
        raise ValueError('Module catalog differs from std/module.builtinModules')
    natives = {key for key, item in catalog.items() if item['mode'] == 'native'}
    if natives != {'native-' + p.stem for p in (ROOT/'stdlib/modules').glob('*.dev')}:
        raise ValueError('Module catalog does not cover native stdlib bindings exactly')
    basic = {
        'io': 'fn write(text str) bool\nfn writeln(text str) bool\nfn read_line() str\nfn read_file(path str) str\nfn write_file(path str, text str) bool',
        'time': 'fn now_ns() u64\nfn now_ms() u64\nfn sleep_ms(milliseconds integer) bool',
        'args': 'fn len() usize\nfn get(index integer) str',
    }
    notes = {
        'io': 'ข้อความใช้ UTF-8; read_line ตัด newline ท้ายบรรทัดออก ความล้มเหลวของ I/O เป็น runtime error; เส้นทาง relative อิง working directory ขณะรัน read_line คืน str โดยไม่ต้องเตรียม buffer ต่างจาก native std/io',
        'time': 'นับเวลาที่ผ่านไปตั้งแต่ runtime เริ่มด้วย monotonic clock ไม่ใช่ Unix timestamp ใช้ std/datetime สำหรับวันที่จริง sleep_ms บล็อก thread ปัจจุบันและคืน true; ค่าติดลบเป็น error',
        'args': 'นับเฉพาะ arguments หลัง -- ไม่รวมชื่อ executable หรือ source file get ใช้ index เริ่มจาก 0; index ติดลบหรือเกินจำนวนเป็น error ตรวจ len ก่อน get หรือใช้ std/cli เมื่อต้องการ parse flags',
        'fs/promises': 'แต่ละคำสั่งเริ่ม worker thread และคืน Task<T> เรียก await(task) เพื่อรับผลหรือ error การ await บล็อก thread ผู้เรียก ไม่ใช่ JavaScript Promise หรือ coroutine event loop ใช้ encoding utf8/utf-8 กับ readFile; paths อิง process working directory',
    }
    output = {}
    for key, item in catalog.items():
        name, mode = item['name'], item['mode']
        source = ROOT / 'docs' / item['source']
        contract = source.read_text(encoding='utf-8')
        slug = key.replace('/', '-')
        path = f'docs/modules/{slug}.md'
        code = (ROOT/item['example']).read_text(encoding='utf-8')
        native = mode == 'native'
        declaration = signatures((ROOT/item['binding']).read_text(encoding='utf-8')) if native else (
            basic[key] if key in basic else signatures(constants[modules[key]]))
        status = ('ต้อง build/link C stdlib; signature ต่างจาก source runtime ดู [Native stdlib](../stdlib.md)' if native else
                  'ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้')
        if key in ('io', 'time', 'strings'):
            status = 'Source runtime builtin; native stdlib มีชื่อเดียวกันแต่ API/ownership ต่างกัน ดู [native counterpart](native-' + key + '.md)'
        if key == 'args':
            status = 'Source runtime builtin; ไม่มี native stdlib binding สำหรับ std/args'
        text = f'# {name}' + (' — Native' if native else '') + f'\n\n{item["description"]}\n\n'
        text += f'[กลับหน้ารวม module](index.md) · **โหมด:** {status}\n\n'
        text += '## การ import และเรียกใช้งาน\n\n```dev\nuse "' + name + '"\n```\n\n'
        text += 'เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `' + name.split('/')[-1] + '.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature\n\n'
        text += '## ชนิดข้อมูลและ signatures\n\nรายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)\n\n'
        if re.search(r'\bid i64\b', declaration):
            text += 'Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API\n\n'
        if key in ('json', 'don'):
            text += 'Value เป็น managed data ที่สร้างผ่าน API ไม่ใช่ empty struct ที่ใช้แทน JSON/DON โดยตรง\n\n'
        text += '```text\n' + declaration + '\n```\n\n'
        if key == 'json':
            text += 'JSON มี dynamic intrinsics เพิ่มจาก shared declarations ข้างต้น เช่น value/stringify/pretty/set/push และ scalar accessors รายการ API เต็มพร้อมชนิดผลลัพธ์อยู่ในตารางด้านล่าง\n\n'
        text += '## ตัวอย่างเริ่มต้น\n\n' + item.get('example_note', 'ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก') + '\n\n```dev\n' + code + '```\n\n'
        if native:
            command = f'python stdlib/build.py\nd build {item["example"]} --release --module-dir std=stdlib/modules --link stdlib/lib/libdevruntime.a -o out/{slug}'
            text += 'Build จาก repository root แล้วเรียก executable ใน out/ (Windows ใช้ .exe):\n\n'
        else:
            command = f'd {item["example"]}\nd {item["example"]} --engine ast'
            text += 'ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:\n\n'
        text += '```sh\n' + command + '\n```\n\n'
        if key == 'args':
            text += 'ส่ง arguments เช่น `d ' + item['example'] + ' -- hello world` จะพิมพ์จำนวน 2\n\n'
        text += '## พฤติกรรม, errors และข้อจำกัด\n\n'
        if key in notes:
            text += notes[key] + '\n\n'
        # Retain shared ownership/error contracts as well as the module-specific API.
        if item['source'] == 'node-core.md':
            shared = contract[:contract.index('## path')]
        else:
            shared = contract.split('\n## ', 1)[0]
        shared = re.sub(r'^# .+\n', '', shared).strip()
        if item['sections']:
            text += links(shared, source) + '\n\n'
            text += '\n\n'.join(links(section(contract, h), source) for h in item['sections']) + '\n\n'
        if key == 'fs/promises':
            text += links(section(contract, 'Node-style filesystem API'), source) + '\n\n'
            text += links(section(contract, 'Original filesystem API'), source) + '\n\n'
        text += '## แหล่งอ้างอิงและการตรวจ\n\n'
        text += f'- [สัญญา API ฉบับรวม](../{item["source"]})\n- [Source ตัวอย่าง](../../{item["example"]})\n'
        if native:
            text += f'- [Native bindings](../../{item["binding"]})\n'
        else:
            text += '- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)\n'
        text += '\nสร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม\n'
        output[path] = text
    index = '# Module API\n\nคู่มือแยกครบทุก builtin ของ source runtime และ native C stdlib จาก implementation ปัจจุบัน แต่ละหน้ามีวิธี import, signatures, types, ตัวอย่าง และ errors/ข้อจำกัด ใช้ source build ล่าสุด เพราะ installer v0.4.0 ยังไม่มี API ใหม่ทั้งหมด\n\n'
    for mode, title in [('runtime', 'Source runtime — 42 modules'), ('native', 'Native C stdlib — 4 bindings')]:
        index += f'## {title}\n\n| Module | ใช้งาน |\n| --- | --- |\n'
        for key, item in sorted(catalog.items()):
            if item['mode'] == mode:
                index += f'| [{item["name"]}]({key.replace("/", "-")}.md) | {item["description"]} |\n'
        index += '\n'
    index += '## เลือกโหมดให้ถูก\n\nRuntime builtin ไม่ต้องระบุ --module-dir หรือ --link ส่วน native ใช้ `--module-dir std=stdlib/modules --link stdlib/lib/libdevruntime.a` หลัง build stdlib อย่าสลับ signature ของ std/io, std/strings และ std/time ระหว่างโหมด ใช้ std/memory เฉพาะ native หรือใช้ Ref/Vec/Map สำหรับ managed data ของภาษา\n\nRuntime errors จับด้วย [std/result](result.md) ได้ในขอบเขตที่ระบุ การ catch ไม่ย้อน side effects และ await บล็อก thread; filesystem/network/process APIs ไม่ใช่ Node.js compatibility หรือ coroutine async I/O\n'
    index += '\n## ดาวน์โหลด\n\n[Module API Markdown ZIP](https://d-osc.github.io/DevLang/downloads/devlang-module-api.zip) รวมหน้ารวมและคู่มือแยก module ส่วน [ตัวอย่างพร้อมผลลัพธ์และ ZIP](https://d-osc.github.io/DevLang/#/examples?category=Module%20API) อยู่ในเว็บไซต์ ใช้ repository checkout เมื่อต้องเปิด source และ aggregate reference links ของ Markdown แบบ local\n'
    output['docs/modules/index.md'] = index
    return catalog, output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--d', type=Path, help='Validate both runtime engines and native examples')
    args = parser.parse_args()
    catalog, output = generate()
    for path, content in output.items():
        file = ROOT/path
        if args.check:
            if not file.is_file() or file.read_text(encoding='utf-8') != content:
                raise ValueError(f'Stale module documentation: {path}')
        else:
            file.write_text(content, encoding='utf-8')
    print(f'{"Verified" if args.check else "Built"} {len(output)} module reference pages')
    if args.d:
        binary = str(args.d.resolve())
        for key, item in catalog.items():
            with tempfile.TemporaryDirectory(prefix='dev-module-doc-') as work:
                file = str(ROOT/item['example'])
                if item['mode'] == 'runtime':
                    env = dict(os.environ, PATH='', DEV_CC='missing-compiler')
                    results = [subprocess.run([binary, file, '--engine', engine], cwd=work, env=env,
                                              capture_output=True, check=True, timeout=20)
                               for engine in ('auto', 'ast')]
                    if results[0].stdout != results[1].stdout:
                        raise ValueError(f'Engine outputs differ for {key}')
                else:
                    executable = str(Path(work)/('example.exe' if os.name == 'nt' else 'example'))
                    subprocess.run([binary, 'build', file, '--release', '--module-dir', 'std=stdlib/modules',
                                    '--link', 'stdlib/lib/libdevruntime.a', '-o', executable], cwd=ROOT,
                                   capture_output=True, check=True, timeout=90)
                    subprocess.run([executable], cwd=work, capture_output=True, check=True, timeout=20)
            print(f'PASS {key} ({item["mode"]})', flush=True)


if __name__ == '__main__':
    main()
