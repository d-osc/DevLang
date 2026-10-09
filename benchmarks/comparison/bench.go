package main

import (
	"bufio"
	"fmt"
	"os"
)

func sumLoop(n uint64) uint64 {
	var total, i uint64
	for i < n {
		total += i & 255
		i += 1
	}
	return total
}

func xorshift(n, seed uint64) uint64 {
	state := seed
	var i uint64
	for i < n {
		state ^= state << 13
		state ^= state >> 7
		state ^= state << 17
		i += 1
	}
	return state
}

func arrayUpdate(n int, seed, rounds uint64) uint64 {
	memory := make([]uint64, n)
	i := 0
	for i < n {
		memory[i] = (seed + uint64(i)) & 0xffffffff
		i += 1
	}
	var round uint64
	for round < rounds {
		i = 0
		for i < n {
			memory[i] = (memory[i]*1664525 + 1013904223) & 0xffffffff
			i += 1
		}
		round += 1
	}
	var total uint64
	i = 0
	for i < n {
		total += memory[i]
		i += 1
	}
	return total
}

func main() {
	reader := bufio.NewReader(os.Stdin)
	var mode, n, seed, rounds uint64
	if _, err := fmt.Fscan(reader, &mode, &n, &seed, &rounds); err != nil {
		panic(err)
	}
	var result uint64
	switch mode {
	case 1:
		result = sumLoop(n)
	case 2:
		result = xorshift(n, seed)
	case 3:
		result = arrayUpdate(int(n), seed, rounds)
	default:
		os.Exit(1)
	}
	fmt.Println(result)
}
