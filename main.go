package main

import (
	"fmt"
	"os"

	"github.com/S1ro1/popcorn-cli/src/cmd"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: popcorn <command>")
		os.Exit(1)
	}

	if os.Getenv("POPCORN_API_URL") == "" {
		fmt.Println("POPCORN_API_URL is not set. Please set it to the URL of the Popcorn API.")
		os.Exit(1)
	}

	if os.Args[1] == "login" {
		cmd.RunAuthentication()
	} else if os.Args[1] == "submit" {
		cmd.RunSubmission()
	} else {
		fmt.Println("Usage: popcorn <command>")
		os.Exit(1)
	}
}
