package main

import (
	"fmt"
	"os"

	"github.com/S1ro1/popcorn-cli/src/cmd"
)

func displayAsciiArt() {
	art := `
 _   __                      _  ______          _   
| | / /                     | | | ___ \        | |  
| |/ /  ___ _ __ _ __   ___ | | | |_/ /  ___  _| |_ 
|    \ / _ \ '__| '_ \ / _ \| | | ___ \ / _ \| | __|
| |\  \  __/ |  | | | |  __/| | | |_/ /| (_) | | |_ 
\_| \_/\___|_|  |_| |_|\___|_/ \____/ \___/|_|\__|
                                                  
    POPCORN CLI - GPU MODE
    
 ┌───────────────────────────────────────┐
 │  ┌─────┐ ┌─────┐ ┌─────┐              │
 │  │ooOoo│ │ooOoo│ │ooOoo│              │▒
 │  │oOOOo│ │oOOOo│ │oOOOo│              │▒
 │  │ooOoo│ │ooOoo│ │ooOoo│   ┌────────┐ │▒
 │  └─────┘ └─────┘ └─────┘   │████████│ │▒
 │                            │████████│ │▒
 │ ┌────────────────────────┐ │████████│ │▒
 │ │                        │ │████████│ │▒
 │ │  POPCORN GPU COMPUTE   │ └────────┘ │▒
 │ │                        │            │▒
 │ └────────────────────────┘            │▒
 │                                       │▒
 └───────────────────────────────────────┘▒
  ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
    ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
`
	fmt.Println(art)
}

func main() {
	
	if os.Getenv("POPCORN_API_URL") == "" {
		fmt.Println("POPCORN_API_URL is not set. Please set it to the URL of the Popcorn API.")
		os.Exit(1)
	}
	displayAsciiArt()
	cmd.Execute()
}
