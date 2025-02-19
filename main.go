package main

import (
	"fmt"
	"os"

	"github.com/S1ro1/popcorn-cli/src/cmd"
)

func main() {
	_, ok := os.LookupEnv("POPCORN_API_URL")
	if !ok {
		fmt.Println("POPCORN_API_URL is not set. Please set it to the URL of the Popcorn API.")
		os.Exit(1)
	}
	cmd.Execute()
}
