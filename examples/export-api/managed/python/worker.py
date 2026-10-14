"""One JSON request on stdin, one JSON response on stdout."""
import json
import sys

sys.stdin.reconfigure(encoding='utf-8')
sys.stdout.reconfigure(encoding='utf-8')
sys.stderr.reconfigure(encoding='utf-8')

try:
    request = json.load(sys.stdin)
    a, b, name = request['a'], request['b'], request['name']
    if type(a) is not int or type(b) is not int or not isinstance(name, str):
        raise ValueError('expected integer a/b and string name')
    if abs(a) > 1000000 or abs(b) > 1000000:
        raise ValueError('a/b outside example range')
    print(json.dumps({'sum': a + b, 'message': 'Hello ' + name}, ensure_ascii=False))
except Exception as error:
    print(str(error), file=sys.stderr)
    sys.exit(2)
