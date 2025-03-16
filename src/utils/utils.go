package utils

import (
	"fmt"
	"os"
	"strings"
)

type PopcornDirectives struct {
	LeaderboardName string
	Gpus            []string
}

func GetPopcornDirectives(filepath string) (*PopcornDirectives, error) {
	var err error = nil
	content, err := os.ReadFile(filepath)

	var gpus []string = []string{}
	var leaderboard_name string = ""

	if err != nil {
		return nil, err
	}

	lines := strings.Split(string(content), "\n")
	for _, line := range lines {
		if !strings.HasPrefix(line, "//") && !strings.HasPrefix(line, "#") {
			continue
		}

		parts := strings.Split(line, " ")
		if parts[0] == "//!POPCORN" || parts[0] == "#!POPCORN" {
			arg := strings.ToLower(parts[1])
			if arg == "gpu" || arg == "gpus" {
				gpus = parts[2:]
			} else if arg == "leaderboard" {
				leaderboard_name = parts[2]
			}
		}
	}

	if len(gpus) > 1 {
		err = fmt.Errorf("multiple GPUs are not yet supported, continue with the first gpu? (%s) [y/N]", gpus[0])
		gpus = []string{gpus[0]}
	}

	return &PopcornDirectives{
		LeaderboardName: leaderboard_name,
		Gpus:            gpus,
	}, err
}

func DisplayAsciiArt() {
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
