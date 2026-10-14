const math = require(require('node:path').resolve(process.argv[2]));
console.log(math.add(20, 22));
console.log(math.scale(2.5, 5));
console.log(math.positive(7));
console.log(math.positive(-7));
console.log(math.name());
// Invalid JS values must be rejected before crossing the C ABI.
for (const call of [() => math.add('wrong', 1), () => math.add(2147483647, 1), () => math.positive(NaN)]) {
  let rejected = false;
  try { call(); } catch (error) { rejected = error instanceof TypeError || error instanceof RangeError; }
  if (!rejected) throw new Error('invalid argument was accepted');
}
