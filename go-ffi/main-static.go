package main

// NOTE: There should be NO space between the comments and the `import "C"` line.

/*
#cgo LDFLAGS: ./lib/libted.a -ldl
#include "./lib/ted.h"
#include <stdio.h>
#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"fmt"
	"net/http"
	"unsafe"

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

func initDkg(c echo.Context) error {
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

	respStr := initFfi(string(jsonString))

	var resp interface{}
	if err := json.Unmarshal([]byte(respStr), &resp); err != nil {
		fmt.Printf("%v", err)
		return err
	}

	return c.JSON(http.StatusOK, resp)
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
	fmt.Printf("commit dkg req %v\n", string(jsonString))

	respStr := commitFfi(string(jsonString))

	var resp interface{}
	if err := json.Unmarshal([]byte(respStr), &resp); err != nil {
		fmt.Printf("%v", err)
		return err
	}

	return c.JSON(http.StatusOK, resp)
}

func finalizeDkg(c echo.Context) error {
	var body interface{}
	if err := (&echo.DefaultBinder{}).BindBody(c, &body); err != nil {
		return err
	}
	jsonString, err := json.Marshal(body)
	if err != nil {
		fmt.Printf("%v", err)
		return err
	}
	fmt.Printf("finalize dkg req %v\n", string(jsonString))

	respStr := finalizeFfi(string(jsonString))

	var resp interface{}
	if err := json.Unmarshal([]byte(respStr), &resp); err != nil {
		fmt.Printf("%v", err)
		return err
	}

	return c.JSON(http.StatusOK, resp)
}

func initFfi(jsonString string) string {
	input := C.CString(jsonString)
	defer C.free(unsafe.Pointer(input))
	o := C.init(input)
	output := C.GoString(o)
	fmt.Printf("init ffi output %s\n", output)
	return output
}

func commitFfi(jsonString string) string {
	input := C.CString(jsonString)
	defer C.free(unsafe.Pointer(input))
	o := C.commit(input)
	output := C.GoString(o)
	fmt.Printf("commit ffi output %s\n", output)
	return output
}

func finalizeFfi(jsonString string) string {
	input := C.CString(jsonString)
	defer C.free(unsafe.Pointer(input))
	o := C.finalize(input)
	output := C.GoString(o)
	fmt.Printf("finalize ffi output %s\n", output)
	return output
}
