import random
import unittest
from benchmark_compare import MASK32, reference, xorshift_step


class ReferenceTests(unittest.TestCase):
    def test_sum_against_direct_iteration(self):
        for n in (0, 1, 255, 256, 257, 1025):
            self.assertEqual(reference(1, n, 0, 0), sum(i & 255 for i in range(n)))

    def test_xorshift_matrix_against_direct_iteration(self):
        rng = random.Random(64)
        for _ in range(30):
            seed = rng.getrandbits(64)
            n = rng.randrange(500)
            value = seed
            for _ in range(n):
                value = xorshift_step(value)
            self.assertEqual(reference(2, n, seed, 0), value)

    def test_affine_lcg_against_direct_array_updates(self):
        rng = random.Random(32)
        for _ in range(30):
            seed = rng.getrandbits(32)
            n, rounds = rng.randrange(100), rng.randrange(100)
            values = [(seed + i) & MASK32 for i in range(n)]
            for _ in range(rounds):
                values = [(v * 1664525 + 1013904223) & MASK32 for v in values]
            self.assertEqual(reference(3, n, seed, rounds), sum(values))


if __name__ == "__main__":
    unittest.main()
