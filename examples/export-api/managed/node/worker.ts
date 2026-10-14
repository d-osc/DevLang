declare const require: (name: string) => { readFileSync(fd: number, encoding: string): string };
declare const process: { exitCode: number };
const fs = require('node:fs');
try {
  const request: { a: number; b: number; name: string } = JSON.parse(fs.readFileSync(0, 'utf8'));
  const { a, b, name } = request;
  if (!Number.isInteger(a) || !Number.isInteger(b) || Math.abs(a) > 1000000 || Math.abs(b) > 1000000 || typeof name !== 'string') {
    throw new Error('expected bounded integer a/b and string name');
  }
  console.log(JSON.stringify({ sum: a + b, message: 'Hello ' + name }));
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 2;
}
