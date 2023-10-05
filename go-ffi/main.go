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
	P0Pk   []uint16      `json:"p0_pk"`
	P0Part []interface{} `json:"p0_part"`
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

	var resp InitDkgResp
	if err := json.Unmarshal([]byte(respStr), &resp); err != nil {
		fmt.Printf("%v", err)
		return err
	}

	return c.JSON(http.StatusOK, resp)
}

type CommitDkgReq struct {
	P1Acks [][]interface{} `json:"p1_acks"`
	P1Part []interface{}   `json:"p1_part"`
}

type CommitDkgResp struct {
	P0Acks [][]interface{} `json:"p0_acks"`
}

func commit(c echo.Context) error {
	var body interface{}
	if err := (&echo.DefaultBinder{}).BindBody(c, &body); err != nil {
		return err
	}
	jsonString, err := json.Marshal(body)
	if err != nil {
		fmt.Printf("%v", err)
		return err
	}
	fmt.Printf("init dkg req %v\n", string(jsonString))

	respStr := commitFfi(string(jsonString))

	var resp interface{}
	if err := json.Unmarshal([]byte(respStr), &resp); err != nil {
		fmt.Printf("%v", err)
		return err
	}

	return c.JSON(http.StatusOK, resp)
}

func finalizeDkg(c echo.Context) error {
	// Handle finalize_dkg route
	return c.String(http.StatusOK, "finalize_dkg")
}

func initFfi(jsonString string) string {
	input := C.CString(jsonString)
	o := C.init(input)
	output := C.GoString(o)
	fmt.Printf("init ffi output %s\n", output)
	return output
}

func commitFfi(jsonString string) string {
	input := C.CString(jsonString)
	o := C.commit(input)
	output := C.GoString(o)
	fmt.Printf("commit ffi output %s\n", output)
	return output
}
