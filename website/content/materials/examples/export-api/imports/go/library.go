package main

/*
#include <stdint.h>
#include <stdbool.h>
*/
import "C"

//export foreign_add
func foreign_add(a, b C.int32_t) C.int32_t { return a + b }

//export foreign_scale
func foreign_scale(value, factor C.double) C.double { return value * factor }

//export foreign_positive
func foreign_positive(value C.int32_t) C.bool { return C.bool(value > 0) }

func main() {}
