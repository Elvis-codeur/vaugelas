//! CPython's `random.Random` — MT19937 with the exact `seed`, `getrandbits`, `_randbelow`, `choice` and `shuffle`
//! behaviour of the standard library (Lib/random.py + Modules/_randommodule.c, Python 3.12).
//!
//! Why a reimplementation: the archived Python data generators are deterministic functions of `random.Random(seed)`.
//! Regenerating their datasets byte for byte from Rust requires the same stream, not merely a random one. The three
//! call sites used by `lexpairs.py` / `product_pairs.py` are `shuffle`, `choice` and (through both) `_randbelow`.
//!
//! The arithmetic is wrapping 32-bit throughout, like the C code. `random()` (53-bit double) and `randrange` are
//! included because they are the natural unit-test companions of the seeding code; generate a golden file with
//! `tools/gen_pyrandom_golden.py` and compare.

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER: u32 = 0x8000_0000;
const LOWER: u32 = 0x7fff_ffff;

/// `random.Random` state for one generator.
pub struct PyRandom {
    mt: [u32; N],
    idx: usize,
}

impl PyRandom {
    /// `random.Random(seed)` for a non-negative integer seed (Python takes the absolute value of an int seed; the
    /// generators here use positive seeds and this constructor asserts that, rather than silently mirroring a mistake).
    pub fn new(seed: u64) -> PyRandom {
        let mut r = PyRandom { mt: [0; N], idx: N };
        r.seed(seed);
        r
    }

    /// `random.seed(int)`: the integer's magnitude becomes the little-endian 32-bit key of `init_by_array`.
    pub fn seed(&mut self, seed: u64) {
        let mut key: Vec<u32> = Vec::new();
        let mut n = seed;
        while n > 0 {
            key.push((n & 0xffff_ffff) as u32);
            n >>= 32;
        }
        if key.is_empty() {
            key.push(0);
        }
        self.init_by_array(&key);
    }

    /// `init_genrand` (the C fallback constants) followed by the standard `init_by_array`.
    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(1965_0218);
        let mut i = 1usize;
        let mut j = 0usize;
        let mut k = N.max(key.len());
        while k > 0 {
            self.mt[i] = (self.mt[i] ^ (self.mt[i - 1] ^ (self.mt[i - 1] >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }
        k = N - 1;
        while k > 0 {
            self.mt[i] = (self.mt[i] ^ (self.mt[i - 1] ^ (self.mt[i - 1] >> 30)).wrapping_mul(1_566_083_941))
                .wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            k -= 1;
        }
        self.mt[0] = 0x8000_0000;
        self.idx = N;
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..N {
            self.mt[i] = 1_812_433_253u32
                .wrapping_mul(self.mt[i - 1] ^ (self.mt[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        self.idx = N;
    }

    fn twist(&mut self) {
        for kk in 0..N {
            let y = (self.mt[kk] & UPPER) | (self.mt[(kk + 1) % N] & LOWER);
            self.mt[kk] = self.mt[(kk + M) % N] ^ (y >> 1) ^ if y & 1 != 0 { MATRIX_A } else { 0 };
        }
        self.idx = 0;
    }

    /// `genrand_uint32` / `random.genrand_uint32`.
    pub fn genrand_u32(&mut self) -> u32 {
        if self.idx >= N {
            self.twist();
        }
        let mut y = self.mt[self.idx];
        self.idx += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// `random.random()`: the 53-bit double from two draws.
    pub fn random(&mut self) -> f64 {
        let a = (self.genrand_u32() >> 5) as f64;
        let b = (self.genrand_u32() >> 6) as f64;
        (a * 67_108_864.0 + b) * (1.0 / 9_007_199_254_740_992.0)
    }

    /// `random.getrandbits(k)` for k <= 64 (the generators need k <= 53; `_randbelow(16)` needs k = 5).
    pub fn getrandbits(&mut self, k: u32) -> u64 {
        assert!(k <= 64, "getrandbits: k = {k} > 64 is not implemented");
        if k == 0 {
            return 0;
        }
        if k <= 32 {
            (self.genrand_u32() >> (32 - k)) as u64
        } else {
            // CPython builds the int from 32-bit words, least significant first; the last word keeps its high bits
            // (Lib: `r >>= (32 - k)` drops the low bits of the final word, not of the first).
            let words = (k - 1) / 32 + 1;
            let mut out = 0u64;
            let mut left = k;
            for i in 0..words {
                let mut r = self.genrand_u32();
                if left < 32 {
                    r >>= 32 - left;
                }
                out |= (r as u64) << (32 * i);
                left -= 32;
            }
            out
        }
    }

    /// `random._randbelow_with_getrandbits(n)`: rejection sampling on `n.bit_length()` bits.
    pub fn randbelow(&mut self, n: u64) -> u64 {
        assert!(n > 0, "randbelow: n must be positive");
        let k = 64 - n.leading_zeros();
        loop {
            let r = self.getrandbits(k);
            if r < n {
                return r;
            }
        }
    }

    /// `random.choice(seq)`: `seq[self._randbelow(len(seq))]`.
    pub fn choice<'a, T>(&mut self, seq: &'a [T]) -> &'a T {
        assert!(!seq.is_empty(), "choice from an empty sequence");
        let i = self.randbelow(seq.len() as u64) as usize;
        &seq[i]
    }

    /// `random.shuffle(x)`: Fisher–Yates descending, `j = _randbelow(i + 1)`.
    pub fn shuffle<T>(&mut self, x: &mut [T]) {
        for i in (1..x.len()).rev() {
            let j = self.randbelow(i as u64 + 1) as usize;
            x.swap(i, j);
        }
    }

    /// `random.randrange(stop)` for a positive stop (int path).
    pub fn randrange(&mut self, stop: u64) -> u64 {
        assert!(stop > 0, "randrange: stop must be positive");
        self.randbelow(stop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The very first outputs of `random.Random(0).random()` (a common cross-check; Python 3.12).
    #[test]
    fn first_draws_seed_zero() {
        let mut r = PyRandom::new(0);
        let got: Vec<f64> = (0..3).map(|_| r.random()).collect();
        assert_eq!(got, vec![0.8444218515250481, 0.7579544029403025, 0.420571580830845]);
    }

    #[test]
    fn randbelow_matches_small_choice() {
        // choice over 4 elements is _randbelow(4) = getrandbits(3) rejection sampling
        let mut r = PyRandom::new(17);
        let xs = ["a", "b", "c", "d"];
        let got: Vec<&str> = (0..8).map(|_| *r.choice(&xs)).collect();
        assert_eq!(got, vec!["d", "c", "c", "c", "b", "c", "a", "a"]);
    }

    #[test]
    fn shuffle_is_python_order() {
        let mut xs: Vec<u32> = (0..10).collect();
        PyRandom::new(1).shuffle(&mut xs);
        assert_eq!(xs, vec![6, 8, 9, 7, 5, 3, 0, 4, 1, 2]);
    }
}
