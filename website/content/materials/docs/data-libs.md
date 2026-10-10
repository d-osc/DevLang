# Data and security source-runtime libraries

Six additional runtime modules: `std/regex`, `std/encoding`, `std/crypto`,
`std/compression`, `std/archive`, `std/uuid`. They require the current source build;
native stdlib and existing installers do not provide them. Arguments are required
and signatures are shared with editor checking. Binary APIs use `Vec<u8>`;
`encoding.encode` or `buffer.toBytes` bridges text/Buffer to binary input.

## Regex

```dev-runtime
use "std/regex"
let pattern = regex.compile("[0-9]+", "")
print(pattern.find("port=3000").text)
print(pattern.replaceAll("a1 b2", "#"))
pattern.close()
```

| API | Contract |
| --- | --- |
| `compile(pattern,flags) Regex` | Compile once; flags i/m/s/U or empty |
| `escape(text) str` | Escape regex metacharacters for literal matching |
| `Regex.test(text) bool` | Whether a match exists anywhere |
| `Regex.find(text) Match` | First match; matched/text/start/end fields |
| `Regex.findAll(text) Vec<str>` | All non-overlapping matches |
| `Regex.captures(text) Vec<str>` | First match: group 0 followed by numbered groups |
| `Regex.split(text) Vec<str>` | Split by pattern; preserves empty entries |
| `Regex.replace(text,replacement) str` | Replace first match |
| `Regex.replaceAll(text,replacement) str` | Replace all matches |
| `Regex.close()` | Release compiled handle |

`i` ignores case, `m` enables line anchors, `s` lets dot match newline, `U` swaps
greediness. Duplicate/unknown flags fail. There is no `g` flag: use `findAll` or
`replaceAll`. Unicode matching is enabled. Look-around and backreferences are not
supported. In Dev source, backslashes must be escaped: `"\\d+"`.

Match offsets are UTF-8 **bytes**, end-exclusive. No match returns matched=false,
text="", start/end=-1. Captures returns an empty vector for no match and empty
strings for unmatched optional groups. Replacement supports `$1`, `$name`,
`${name}`, `${1}` and `$$`; unknown groups expand to empty. Use braces before a
suffix (`${1}suffix`). There are no replacement callbacks.

Limits: 32 regex handles per interpreter (also counted in the 256 core handle
limit), 64 KiB pattern/replacement, 2 MiB compiled regex/DFA cache limits, 8 MiB
input/output, 4096 result items. Captured text and replacements are checked before
copying expanded data. Handles belong to their creating interpreter/thread.

## Encoding

| API | Contract |
| --- | --- |
| `encode(text,charset) Vec<u8>` | Strict text encoding; unrepresentable text fails |
| `decode(bytes,charset) str` | Strict decoding; malformed sequences fail |
| `transcode(bytes,from,to) Vec<u8>` | Decode then encode |
| `supported(charset) bool` | Check recognized charset label |

UTF-8, UTF-16LE/BE, Windows-1252, Shift_JIS, GBK, Big5 and other encoding_rs labels
are supported. UTF-16 encoding writes no BOM; decoding preserves an explicit BOM
as U+FEFF, requires even byte length and rejects invalid surrogates. Other decoding
also preserves BOMs. Labels follow WHATWG mappings: `latin1`/`iso-8859-1` map to
Windows-1252, not a strict ISO-8859-1 implementation. Outputs/inputs are capped at
8 MiB. There is no silent replacement of malformed/unrepresentable text.

## Crypto

| API | Contract |
| --- | --- |
| `hash(algorithm,bytes) Vec<u8>` | SHA-256 or SHA-512 digest |
| `hashText(algorithm,text) str` | UTF-8 digest as lowercase hex |
| `hmac(algorithm,key,data) Vec<u8>` | HMAC SHA-256/SHA-512 |
| `verifyHmac(algorithm,key,data,tag) bool` | Verify tag; wrong tag/length returns false |
| `secureBytes(size) Vec<u8>` | OS-provided cryptographic random bytes |
| `timingSafeEqual(a,b) bool` | Equal-length content comparison using subtle; differing lengths return false |
| `encrypt(key,data,associatedData) Vec<u8>` | AES-256-GCM with fresh random nonce |
| `decrypt(key,packet,associatedData) Vec<u8>` | Authenticate before returning plaintext |

Algorithm names are exactly `sha256` / `sha512`. AES keys must contain 32 bytes;
generate with `secureBytes(32)`. This is distinct from the noncryptographic
`std/random` PRNG. AES-GCM uses the RustCrypto implementation; algorithms are not
implemented from scratch in DevLang.

```dev-runtime
use "std/crypto"
use "std/encoding"
let key = crypto.secureBytes(32)
let data = encoding.encode("DevLang", "utf8")
let aad = encoding.encode("example-v1", "utf8")
let packet = crypto.encrypt(key, data, aad)
print(encoding.decode(crypto.decrypt(key, packet, aad), "utf8"))
```

Packet format: **12-byte nonce + ciphertext + 16-byte authentication tag**.
Encrypt generates the nonce through the OS; callers cannot specify it. Decrypt
requires the same key and associated data. A modified packet/key/AAD raises an
authentication error. Associated data is authenticated but not included in the
packet, so its storage/protocol is the application's responsibility. Empty AAD
can be supplied as `Vec<u8>()`.

The complete encrypted packet is capped at 8 MiB (plaintext maximum 8 MiB minus
28 bytes). Other binary inputs/outputs are capped at 8 MiB. Lengths are visible;
timingSafeEqual does not make surrounding interpreted code constant-time.
Managed values/copies are not automatically zeroized. No password hashing/KDF,
RSA/ECC signatures, key storage or certificate APIs are implemented in this set;
raw passwords are not AES keys and SHA digests are not password-storage APIs.
No system-level security certification is claimed by these runtime tests.

## Compression

| API | Contract |
| --- | --- |
| `gzip(bytes,level)` / `gunzip(bytes)` | GZIP; decode supports concatenated members |
| `zlib(bytes,level)` / `unzlib(bytes)` | ZLIB-wrapped DEFLATE |
| `deflate(bytes,level)` / `inflate(bytes)` | Raw DEFLATE |

All outputs are Vec<u8>. Level is 0..9. Input, encoded output and decoded output
are limited to 8 MiB. Decoders enforce output limits while reading, reject
truncated/malformed streams and trailing invalid data. GZIP/ZLIB checksums are
validated; raw DEFLATE has no checksum. These APIs are one-shot, not streaming;
there is no Brotli/Zstd module yet.

## Archive

`archive.Entry(name str,data Vec<u8>)` represents a file. Build a
`Vec<archive.Entry>` and call writeZIP/writeTAR.

| API | Contract |
| --- | --- |
| `writeZIP(entries)` / `writeTAR(entries)` | Create archive bytes |
| `listZIP(bytes)` / `listTAR(bytes)` | List regular-file names |
| `readZIP(bytes,name)` / `readTAR(bytes,name)` | Read one named file as Vec<u8> |

```dev-runtime
use "std/archive"
use "std/encoding"
let entries = Vec<archive.Entry>()
entries.push(archive.Entry("hello.txt", encoding.encode("hello", "utf8")))
let packed = archive.writeZIP(entries)
print(encoding.decode(archive.readZIP(packed, "hello.txt"), "utf8"))
```

ZIP creation uses DEFLATE; reading supports stored/DEFLATE files. TAR creates
regular GNU-format headers with deterministic timestamps/permissions. To combine
TAR and GZIP, pass writeTAR output into compression.gzip. To save/read archives
as files, use `fs.write_bytes` / `fs.read_bytes`.

Names must be UTF-8 relative slash-separated paths, without empty/dot/parent
components, backslashes, drive colons or NUL; maximum 1024 bytes. Duplicate names,
links, special files and encrypted members are rejected. Directory members are
validated but omitted from listing. Missing files raise errors. Maximum 4096
entries, 8 MiB encoded input/output and 8 MiB total declared file data.
Read operations also bound actual expanded member data. ZIP member CRC is checked
when that member is read; listing is metadata inspection, not a full integrity
scan of every payload. There is **no filesystem extraction** or folder-recursion
API, and no password-protected ZIP, RAR or 7z support.

## UUID

| API | Contract |
| --- | --- |
| `v4() str` | Random RFC UUID version 4, using OS entropy |
| `parse(text) str` | Normalize a parseable UUID to lowercase hyphenated form |
| `isValid(text) bool` | Whether the UUID crate accepts the representation |
| `version(text) i64` | Version nibble (nil is 0) |
| `nil() str` | All-zero UUID |

UUIDs are represented as str; generation is limited to v4 in this release.
Parsing does not require the UUID to be v4. There is no v7 generator yet.

## Validation

```sh
d examples/data-libs/main.dev
python scripts/smoke_data_libs.py --bin-dir out/core-io/release
```

The test suite uses empty PATH and no C compiler for Dev execution. It compares
hash/HMAC to Python hashlib/hmac, encodings/compression/archives to Python tools,
and AES-GCM in both directions with Node when available. It tests tampering,
truncation, output limits, regex handles and unsafe archive names. It does not
constitute a security audit or prove all malformed inputs are handled.
