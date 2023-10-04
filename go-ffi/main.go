package main

// NOTE: There should be NO space between the comments and the `import "C"` line.

/*
#cgo LDFLAGS: -L./lib -lted
#include "./lib/ted.h"
*/
import "C"
import (
	"fmt"
)

func main() {
	jsonString := `{"p1_pk":[137,228,198,125,28,60,110,83,86,246,132,115,103,140,234,233,155,39,65,49,0,251,172,228,253,201,213,180,152,13,10,77,255,251,16,173,92,60,76,165,241,76,198,97,122,252,17,104]}`

	input := C.CString(jsonString)

	o := C.init(input)
	output := C.GoString(o)
	fmt.Printf("Second time %s\n", output)
}
