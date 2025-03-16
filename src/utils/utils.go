package utils

import (
	"os"
	"strings"
)

type PopcornDirectives struct {
	LeaderboardName string
	Gpus            []string
}

func GetPopcornDirectives(filepath string) (*PopcornDirectives, error) {
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

	return &PopcornDirectives{
		LeaderboardName: leaderboard_name,
		Gpus:            gpus,
	}, nil
}

