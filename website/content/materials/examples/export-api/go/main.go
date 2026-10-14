package main

/*
#cgo CFLAGS: -I${SRCDIR}/..
#cgo LDFLAGS: -L${SRCDIR}/../../../out/export-api -ldevmath
#include "devmath.h"
*/
import "C"
import "fmt"

func main() {
    fmt.Println(int32(C.devmath_add(20, 22)))
    fmt.Println(float64(C.devmath_scale(2.5, 5.0)))
    fmt.Println(bool(C.devmath_positive(7)))
    // GoString copies the borrowed C string; do not free the DevLang literal.
    fmt.Println(C.GoString(C.devmath_name()))
}
