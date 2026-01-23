package main

import (
	"encoding/base64"
	"fmt"
)

func main() {
	s := "gtLyJqFVtWUzgPrqQo/thRSS3lzLj6/gRBTx7Ccdm1Ko="
	b, err := base64.StdEncoding.DecodeString(s)
	if err != nil {
		fmt.Printf("Error decoding: %v\n", err)
		return
	}
	fmt.Printf("Decoded length: %d\n", len(b))
	if len(b) != 32 {
		fmt.Println("FAIL: Length is not 32")
	} else {
		fmt.Println("SUCCESS: Key is valid")
	}
}
