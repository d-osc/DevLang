const fs = require('node:fs');
try {
  const request = JSON.parse(fs.readFileSync(0, 'utf8'));
  const { a, b, name } = request;
  if (!Number.isInteger(a) || !Number.isInteger(b) || Math.abs(a) > 1000000 || Math.abs(b) > 1000000 || typeof name !== 'string') {
    throw new Error('expected bounded integer a/b and string name');
  }
  console.log(JSON.stringify({ sum: a + b, message: 'Hello ' + name }));
} catch (error) {
  console.error(error.message);
  process.exitCode = 2;
}
