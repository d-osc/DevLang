"""Check SQLite and text data formats against independent Python stdlib fixtures."""
import argparse
import csv
from contextlib import closing
import io
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import tomllib


def literal(text):
    # Dev's lexer supports \n/\r/\t/\0, not JSON's general \uXXXX escapes.
    return json.dumps(text, ensure_ascii=False).replace('\\u0000', '\\0')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', required=True)
    args = parser.parse_args()
    d = Path(args.bin_dir).resolve() / ('d.exe' if os.name == 'nt' else 'd')
    count = 0
    with tempfile.TemporaryDirectory(prefix='dev-storage-') as temporary:
        root = Path(temporary)
        source = root / 'main.dev'
        env = dict(os.environ, PATH='', DEV_CC='missing-compiler')

        def run(code, expected=None, error=None, check=None):
            nonlocal count
            source.write_text(code, encoding='utf-8')
            for engine in ('auto', 'ast'):
                result = subprocess.run([str(d), str(source), '--engine', engine], cwd=root,
                                        env=env, capture_output=True, text=True, encoding='utf-8', timeout=25)
                if error:
                    assert result.returncode != 0 and error in result.stderr, (code, result.stdout, result.stderr)
                else:
                    assert result.returncode == 0, (code, result.stdout, result.stderr)
                    if expected is not None:
                        assert result.stdout == expected, (code, result.stdout, result.stderr)
                if check:
                    check(result)
                count += 1

        run('''use "std/sqlite"
use "std/encoding"
fn main() {
let db=sqlite.open(":memory:")
print(db.execute("CREATE TABLE values_table(n INTEGER, r REAL, t TEXT, b BLOB, x)",Vec<sqlite.Value>()))
let params=Vec<sqlite.Value>()
params.push(sqlite.Value.Integer(-9223372036854775808))
params.push(sqlite.Value.Real(1.25))
params.push(sqlite.Value.Text("ไทย 🚀; DROP TABLE values_table;"))
params.push(sqlite.Value.Blob(encoding.encode("bytes","utf8")))
params.push(sqlite.Value.Null())
print(db.execute("INSERT INTO values_table VALUES (?,?,?,?,?)",params))
print(db.lastInsertRowId())
let data=db.query("SELECT n,r,t,b,x FROM values_table",Vec<sqlite.Value>())
print(data.columns.len())
print(data.rows.len())
for i in 0..5 {
    match data.rows[0][i as usize] {
        Null => { print("null") }
        Integer(n) => { print(n) }
        Real(n) => { print(n) }
        Text(s) => { print(s) }
        Blob(b) => { print(encoding.decode(b,"utf8")) }
    }
}
db.close()
}
main()
''', '0\n1\n1\n5\n1\n-9223372036854775808\n1.25\nไทย 🚀; DROP TABLE values_table;\nbytes\nnull\n')
        run('''use "std/sqlite"
use "std/result"
fn main() {
let db=sqlite.open(":memory:")
let params=Vec<sqlite.Value>()
db.execute("CREATE TABLE items(id INTEGER UNIQUE)",params)
db.execute("BEGIN",params)
db.execute("INSERT INTO items VALUES (1)",params)
let bad=result.run(fn() { db.execute("INSERT INTO items VALUES (1)",params) })
print(result.isErr(bad))
db.execute("ROLLBACK",params)
print(db.query("SELECT * FROM items",params).rows.len())
db.execute("BEGIN",params)
db.execute("INSERT INTO items VALUES (2)",params)
db.execute("COMMIT",params)
print(db.query("SELECT * FROM items",params).rows.len())
db.close()
let closed=result.run(fn() { db.query("SELECT 1",params) })
print(result.isErr(closed))
}
main()
''', 'true\n0\n1\ntrue\n')
        run('''use "std/sqlite"
use "std/result"
fn main() {
let db=sqlite.open(":memory:")
let params=Vec<sqlite.Value>()
db.setTimeout(10)
let slow=result.run(fn() { db.query("WITH RECURSIVE numbers(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM numbers WHERE x<1000000000) SELECT SUM(x) FROM numbers",params) })
print(result.isErr(slow))
print(db.query("SELECT 1",params).rows.len())
db.close()
}
main()
''', 'true\n1\n')
        db_prefix = 'use "std/sqlite"\nlet db=sqlite.open(":memory:")\n'
        for code, error in [
            ('db.query("SELECT 1; SELECT 2",Vec<sqlite.Value>())', 'Multiple statements'),
            ('db.execute("SELECT 1",Vec<sqlite.Value>())', 'Execute returned results'),
            ('db.query("SELECT ?",Vec<sqlite.Value>())', 'Wrong number of parameters'),
            ('db.query("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<4097) SELECT x FROM n",Vec<sqlite.Value>())', '4096 rows'),
            ('db.query("SELECT zeroblob(8388609)",Vec<sqlite.Value>())', 'too big'),
            ('db.query("SELECT CAST(x\'FF\' AS TEXT)",Vec<sqlite.Value>())', 'not UTF-8'),
            ('db.setTimeout(0)', 'at least 1'),
            ('db.setTimeout(300001)', 'exceeds'),
            ('db.query("",Vec<sqlite.Value>())', '1..65536'),
        ]:
            run(db_prefix + code, error=error)
        run(db_prefix + '''let params=Vec<sqlite.Value>()
for i in 0..1000 { params.push(sqlite.Value.Null()) }
db.query("SELECT ?",params)
''', error='999 items')
        run('''use "std/sqlite"
use "std/result"
let handles=Vec<sqlite.Database>()
for i in 0..16 { handles.push(sqlite.open(":memory:")) }
print(result.isErr(result.attempt(fn() sqlite.Database { return sqlite.open(":memory:") })))
for i in 0..16 { handles[i as usize].close() }
let recovered=sqlite.open(":memory:")
print(recovered.query("SELECT 1",Vec<sqlite.Value>()).rows.len())
recovered.close()
''', 'true\n1\n')
        database = root / 'interop.sqlite'
        with closing(sqlite3.connect(database)) as connection, connection:
            connection.execute('CREATE TABLE data (name TEXT, bytes BLOB)')
            connection.execute('INSERT INTO data VALUES (?,?)', ('Python', b'\x00\xff'))

        def check_database(result):
            with closing(sqlite3.connect(database)) as connection, connection:
                assert connection.execute('SELECT name,bytes FROM data ORDER BY rowid').fetchall() == [('Python', b'\x00\xff'), ('Dev', b'ok')]
                connection.execute('DELETE FROM data WHERE name=?', ('Dev',))

        run('''use "std/sqlite"
use "std/encoding"
fn main() {
let db=sqlite.open(PATH)
let data=db.query("SELECT name,bytes FROM data",Vec<sqlite.Value>())
match data.rows[0][0] { Text(value) => { print(value) } _ => {} }
match data.rows[0][1] { Blob(value) => { print(value[1]) } _ => {} }
let params=Vec<sqlite.Value>()
params.push(sqlite.Value.Text("Dev"))
params.push(sqlite.Value.Blob(encoding.encode("ok","utf8")))
db.execute("INSERT INTO data VALUES (?,?)",params)
db.close()
}
main()
'''.replace('PATH', literal(str(database))), 'Python\n255\n', check=check_database)

        records = [['name', 'note'], ['ไทย 🚀', 'comma, and "quotes"'], ['newline', 'line1\nline2']]
        fixture = io.StringIO(newline='')
        csv.writer(fixture).writerows(records)
        text = fixture.getvalue()
        run('''use "std/csv"
use "std/fs"
let table=csv.parse(TEXT,",",true)
print(table.headers[0])
print(table.rows[0][0])
print(table.rows[0][1])
print(table.rows[1][1])
fs.write_text("output.csv",csv.stringify(table,","))
'''.replace('TEXT', literal(text)), 'name\nไทย 🚀\ncomma, and "quotes"\nline1\nline2\n',
            check=lambda result: (_ for _ in ()).throw(AssertionError('CSV interoperability'))
            if list(csv.reader(io.StringIO((root/'output.csv').read_text(encoding='utf-8'), newline=''))) != records else None)
        run('use "std/csv"\nlet data=csv.parse('+literal('a\tb\n1\t2\n')+',"\\t",false)\nprint(data.headers.len())\nprint(data.rows[1][1])', '0\n2\n')
        run('use "std/csv"\nlet data=csv.parse("",",",true)\nprint(data.rows.len())\nprint(csv.stringify(data,",") == "")', '0\ntrue\n')
        for delimiter in ('', '||', 'é', '"', '\n', '\0'):
            run('use "std/csv"\ncsv.parse("a,b",' + literal(delimiter) + ',false)', error='CSV delimiter')
        run('use "std/csv"\ncsv.parse('+literal('a,b\n1\n')+',",",true)', error='found record with')
        run('use "std/csv"\ncsv.parse("a\\0b",",",false)', error='NUL')
        wide = ','.join('a' for _ in range(257))
        run('use "std/csv"\ncsv.parse(' + literal(wide) + ',",",false)', error='256 columns')
        tall = 'a\n' * 4097
        run('use "std/csv"\ncsv.parse(' + literal(tall) + ',",",false)', error='4096 rows')
        run('''use "std/csv"
let headers=Vec<str>()
headers.push("name")
let rows=Vec<Vec<str>>()
let row=Vec<str>()
row.push("x")
row.push("y")
rows.push(row)
csv.stringify(csv.Table(headers,rows),",")
''', error='unequal lengths')

        toml_fixture = "name = 'DevLang'\nenabled = true\nports = [80,443]\n[server]\nhost = 'localhost'\n"
        model = tomllib.loads(toml_fixture)
        run('''use "std/toml"
use "std/fs"
print(toml.toJSON(TEXT))
fs.write_text("output.toml",toml.fromJSON(MODEL))
'''.replace('TEXT', literal(toml_fixture)).replace('MODEL', literal(json.dumps(model, ensure_ascii=False))),
            json.dumps(model, sort_keys=True, separators=(',', ':'), ensure_ascii=False)+'\n',
            check=lambda result: (_ for _ in ()).throw(AssertionError('TOML interoperability'))
            if tomllib.loads((root/'output.toml').read_text(encoding='utf-8')) != model else None)
        run('''use "std/toml"
use "std/json"
let data=toml.parse("date=1979-05-27T07:32:00Z\\n")
print(json.string(json.get(data,"date")))
print(toml.valid("x=1\\nx=2"))
print(toml.valid(""))
''', '1979-05-27T07:32:00Z\nfalse\ntrue\n')
        for text, error in [('{"x":null}', 'no null'), ('[1,2]', 'root must be'), ('{"x":18446744073709551615}', 'outside i64')]:
            run('use "std/toml"\ntoml.fromJSON(' + literal(text) + ')', error=error)
        run('use "std/toml"\ntoml.parse("x=nan")', error='non-finite')
        run('''use "std/toml"
struct Config { name str, enabled bool }
print(toml.toJSON(toml.stringify(Config("Dev",true))))
''', '{"enabled":true,"name":"Dev"}\n')

        yaml_fixture = 'name: DevLang\nenabled: true\nports: [80, 443]\nserver:\n  host: localhost\n'
        run('''use "std/yaml"
use "std/json"
let data=yaml.parse(TEXT)
print(json.string(json.get(data,"name")))
print(yaml.toJSON(yaml.fromJSON(MODEL)))
'''.replace('TEXT', literal(yaml_fixture)).replace('MODEL', literal(json.dumps(model, ensure_ascii=False))),
            'DevLang\n'+json.dumps(model, sort_keys=True, separators=(',', ':'), ensure_ascii=False)+'\n')
        run('use "std/yaml"\nprint(yaml.toJSON('+literal('a: &value [1,2]\nb: *value\n')+'))', '{"a":[1,2],"b":[1,2]}\n')
        run('use "std/yaml"\nprint(yaml.toJSON('+literal('message: |\n  hello\n  world\n')+'))', '{"message":"hello\\nworld\\n"}\n')
        run('use "std/yaml"\nprint(yaml.toJSON(yaml.fromJSON("18446744073709551615")))', '18446744073709551615\n')
        for text, error in [
            ('a: 1\na: 2', 'duplicate'), ('---\na: 1\n---\nb: 2', 'YAML'),
            ('a: .nan', 'YAML'), ('a: !include secret.txt', 'YAML'),
            ('a: &base {x: 1}\nb: {<<: *base}', 'YAML'),
            ('a: &loop [*loop]', 'YAML'),
            ('x: ['*130+'0'+']'*130, 'YAML')]:
            run('use "std/yaml"\nyaml.parse(' + literal(text) + ')', error=error)
        aliases = 'a: &value text\nb: [' + ','.join('*value' for _ in range(65)) + ']'
        run('use "std/yaml"\nyaml.parse(' + literal(aliases) + ')', error='YAML')
        run('use "std/yaml"\nprint(yaml.valid("x: ["))\nprint(yaml.valid("x: true"))', 'false\ntrue\n')
        run('''use "std/yaml"
struct Config { name str, enabled bool }
print(yaml.toJSON(yaml.stringify(Config("Dev",true))))
''', '{"enabled":true,"name":"Dev"}\n')
    print(f'PASS: {count} storage-library executions across auto/AST; SQLite/CSV/TOML Python interoperability verified')


if __name__ == '__main__':
    main()
