package main

import (
	"fmt"
	"os"

	"github.com/S1ro1/popcorn-cli/src/cmd"
)

func main() {
	if os.Getenv("POPCORN_API_URL") == "" {
		fmt.Println("POPCORN_API_URL is not set. Please set it to the URL of the Popcorn API.")
		os.Exit(1)
	}
	cmd.Execute()
}
