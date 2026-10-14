interface DevMath {
  add(a: number, b: number): number;
  scale(value: number, factor: number): number;
  positive(value: number): boolean;
  name(): string;
}
declare function require(name: "node:path"): { resolve(path: string): string };
declare function require(name: string): DevMath;
declare const process: { argv: string[] };
const math: DevMath = require(require('node:path').resolve(process.argv[2]));
console.log(math.add(20, 22));
console.log(math.scale(2.5, 5));
console.log(math.positive(7));
console.log(math.positive(-7));
console.log(math.name());
