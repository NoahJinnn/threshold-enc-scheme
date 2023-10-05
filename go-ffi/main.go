package main

// NOTE: There should be NO space between the comments and the `import "C"` line.

/*
#cgo LDFLAGS: -L./lib -lted
#include "./lib/ted.h"
*/
import "C"
import (
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/labstack/echo/v4"
)

func main() {
	// jsonString := `{"p1_pk":[137,228,198,125,28,60,110,83,86,246,132,115,103,140,234,233,155,39,65,49,0,251,172,228,253,201,213,180,152,13,10,77,255,251,16,173,92,60,76,165,241,76,198,97,122,252,17,104]}`
	// input := C.CString(jsonString)
	// o := C.init(input)
	// output := C.GoString(o)
	// fmt.Printf("Second time %s\n", output)
	e := echo.New()
	// Routes
	e.POST("/init_dkg", initDkg)
	e.POST("/commit", commit)
	e.POST("/finalize_dkg", finalizeDkg)
	e.Logger.Fatal(e.Start(":3002"))

}

type InitDkgReq struct {
	P1Pk []uint16 `json:"p1_pk"`
}

type InitDkgResp struct {
	P0Pk   []byte `json:"p0_pk"`
	P0Part Part   `json:"p0_part"`
}

type Part struct {
	Degree int      `json:"degree"`
	Coeff  [][]byte `json:"coeff"`
}

func initDkg(c echo.Context) error {
	var body InitDkgReq
	if err := (&echo.DefaultBinder{}).BindBody(c, &body); err != nil {
		return err
	}
	jsonString, err := json.Marshal(body)
	if err != nil {
		fmt.Printf("%v", err)
		return err
	}
	fmt.Printf("init dkg req %v\n", string(jsonString))

	respStr := initFfi(string(jsonString))

	// var resp *InitDkgResp
	// json.Unmarshal([]byte(respStr), &resp)
	// fmt.Printf("init dkg resp %v\n", resp)
	var resp InitDkgResp
	if err := json.Unmarshal([]byte(respStr), &resp); err != nil {
		fmt.Printf("%v", err)
		return err
	}
	fmt.Printf("init dkg resp %v\n", respStr)

	return c.JSON(http.StatusOK, resp)
}

func commit(c echo.Context) error {
	// Handle commit route
	return c.String(http.StatusOK, "commit")
}

func finalizeDkg(c echo.Context) error {
	// Handle finalize_dkg route
	return c.String(http.StatusOK, "finalize_dkg")
}

func initFfi(jsonString string) string {
	input := C.CString(jsonString)
	o := C.init(input)
	output := C.GoString(o)
	fmt.Printf("ffi output %s\n", output)
	return output
}
