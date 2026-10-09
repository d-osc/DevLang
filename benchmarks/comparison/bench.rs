use std::io::{self, Read};

fn sum_loop(n: u64) -> u64 {
    let mut total = 0u64;
    let mut i = 0u64;
    while i < n {
        total += i & 255;
        i += 1;
    }
    total
}

fn xorshift(n: u64, seed: u64) -> u64 {
    let mut state = seed;
    let mut i = 0u64;
    while i < n {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        i += 1;
    }
    state
}

fn array_update(n: usize, seed: u64, rounds: u64) -> u64 {
    let mut memory = vec![0u64; n];
    let mut i = 0usize;
    while i < n {
        memory[i] = (seed + i as u64) & 0xffffffff;
        i += 1;
    }
    let mut round = 0u64;
    while round < rounds {
        i = 0;
        while i < n {
            memory[i] = (memory[i] * 1664525 + 1013904223) & 0xffffffff;
            i += 1;
        }
        round += 1;
    }
    let mut total = 0u64;
    i = 0;
    while i < n {
        total += memory[i];
        i += 1;
    }
    total
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let values: Vec<u64> = input
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    let result = match values[0] {
        1 => sum_loop(values[1]),
        2 => xorshift(values[1], values[2]),
        3 => array_update(values[1] as usize, values[2], values[3]),
        _ => std::process::exit(1),
    };
    println!("{result}");
}
