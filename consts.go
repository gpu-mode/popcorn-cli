package main

import (
	"github.com/charmbracelet/bubbles/list"
)

const BASE_URL = "http://localhost:8000"

var runnerItems = []list.Item{
	runnerItem{title: "Modal", description: "Submit to Modal", value: "modal"},
	runnerItem{title: "Github", description: "Submit to Github", value: "github"},
}

var submissionModeItems = []list.Item{
	submissionModeItem{title: "Test", description: "Test the solution"},
	submissionModeItem{title: "Benchmark", description: "Benchmark the solution"},
	submissionModeItem{title: "Leaderboard", description: "Submit to the leaderboard"},
}

var gpuItems = []list.Item{
	gpuItem{title: "A100"},
	gpuItem{title: "A10G"},
	gpuItem{title: "A100"},
	gpuItem{title: "A10G"},
}
