# DNS and command-line libraries

`std/dns` and `std/cli` are built into the updated **source runtime**. Their
signatures are shared with the Language Server. Native builds do not implement
these modules. They need no C compiler or external executable.

## DNS / host resolution

```dev
use "std/dns"
fn main() {
    let addresses = dns.lookup("localhost", 0)
    for i in 0..addresses.len() as i64 {
        print(addresses[i as usize])
    }
    let first = dns.lookupOne("127.0.0.1", 4)
    print(first.address)
    print(first.family)
}
main()
```

| API | Result |
| --- | --- |
| `lookup(host str, family i64)` | `Vec<str>` of canonical IP addresses |
| `lookupOne(host str, family i64)` | `Address { address str, family i64 }`, first matching result |
| `isIP(address str)` | `i64`: 4 for IPv4, 6 for IPv6, 0 for invalid input |
| `isIPv4(address str)` | `bool` |
| `isIPv6(address str)` | `bool` |

`family` must be 0 (either), 4 or 6. Duplicate IPs are removed while preserving
system resolver order. `lookup` returns an empty vector if resolution succeeds
but the selected family has no results; `lookupOne` reports a located error.
Resolver failures also report located runtime errors, not empty success.

Numeric IP literals bypass name resolution and are normalized; `::1` is valid,
`[::1]` and `127.0.0.1:80` are not IP inputs. Scoped IPv6 addresses are not
supported. Hostnames must use ASCII letters/digits/hyphens, labels of 1..63
bytes, and at most 253 bytes overall. Labels cannot start/end with a hyphen;
a trailing root dot is allowed. For international domains use
`url.domainToASCII(name)` before lookup. IP classifiers simply return false/0
for invalid input without attempting a lookup.

This is **OS host resolution**, including hosts-file and OS resolver policy,
not a raw DNS-record client. It has no MX/TXT/SRV/PTR, TTL, reverse lookup,
custom DNS server, DNSSEC validation, cancellation or configurable timeout.
Lookups block until the system resolver returns; its ordering/cache/timeouts
are controlled by the OS. At most 256 matching unique IPs are returned; this
does not limit the resolver's internal allocations. No network socket remains
open after the call.

Use the returned address with `net.connect(port,address,timeout_ms)` or UDP
APIs. A connect timeout does not bound a separate DNS lookup. For concurrent
work, call lookup inside an existing `async fn` and await its Task; that uses an
OS thread and does not add a cancellable resolver or event loop.

## CLI argument parsing

```dev
use "std/cli"
fn main() {
    let options = Vec<cli.Option>()
    options.push(cli.Option("verbose", "v", false))
    options.push(cli.Option("port", "p", true))
    let parsed = cli.parse(cli.args(), options)
    if parsed.flags.contains("verbose") { print("verbose enabled") }
    if parsed.values.contains("port") { print(parsed.values.get("port")) }
    for i in 0..parsed.positionals.len() as i64 {
        print(parsed.positionals[i as usize])
    }
}
main()
```

```sh
d main.dev -- -vp3000 serve -- --literal
```

The first `--` belongs to `d` and forwards program arguments. The second belongs
to `cli.parse` and ends option parsing, making `--literal` positional.

| API/type | Contract |
| --- | --- |
| `args()` | `Vec<str>` of program arguments only; excludes CLI executable and source path |
| `Option(name str, short str, takesValue bool)` | Declares long name, optional one-letter alias, and whether a value is required |
| `parse(args Vec<str>, options Vec<Option>)` | `Parsed { values Map<str,str>, flags Map<str,bool>, positionals Vec<str> }` |

Map keys are the canonical long names, irrespective of the alias used. Present
flags contain `true`; absent flags/values have no map entry. Check `.contains`
before `.get`, because a missing-key Map.get is an error. Values stay strings:
parse and validate numeric values with your application's rules.

Supported forms with the example schema:

| Input | Meaning |
| --- | --- |
| `--verbose`, `-v` | Flag |
| `--port 3000`, `--port=3000` | Value option |
| `-p 3000`, `-p3000`, `-p=3000` | Short value option |
| `-vp3000` | Grouped flag followed by value option |
| `--port -1` | The value is the string `-1` |
| `--port=` | Explicit empty string value |
| `serve --verbose file.dev` | Options interleaved with positionals |
| `-- --verbose -p3000` | Both following tokens are positional |
| `-` | Positional, e.g. stdin marker in your application |

A value option consumes the next token verbatim, even if it starts with `-`:
`--port --verbose` sets port to `--verbose` and does not enable the flag.
An isolated `--` is never consumed as a separated value; use `--port=--` or
`-p=--` for that literal. Attached values consume the remainder of a short
group, so `-pv` means port `v`, not two flags. Quoting is handled by the calling
shell; the parser receives already separated tokens and preserves Unicode.

Unknown options, repeated options (including long/short aliases), missing
values, and values attached to long flags are errors. There is no implicit
`--help`, `--version`, `--no-*`, optional value, abbreviations or repeatable
option. Define help/version flags explicitly and print your own message; the
library never exits the process. Subcommands can be interpreted from
`positionals`; no automatic command dispatcher is provided.

Option names start with an ASCII letter, contain only letters/digits/hyphens,
and are at most 64 bytes. A short alias is empty or one ASCII letter; it is
case-sensitive. Duplicate names/aliases are rejected even if unused. Parsing
allows at most 128 definitions, 4096 arguments and 8 MiB of argument text;
embedded NUL is rejected. These limits apply to args/parse, not to all runtime
strings or all OS argument APIs.

## Validation

Run `d examples/system-libs/main.dev` for a deterministic example.
`python scripts/smoke_system_libs.py --bin-dir out/core-io/release` exercises both
source engines with an empty PATH and a missing C compiler. Tests compare
localhost results with Python's OS resolver and open a real loopback TCP
connection using a resolved address; no public internet service is required.
