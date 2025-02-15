package main

import (
	"github.com/charmbracelet/bubbles/list"
)

const BASE_URL = "http://localhost:8000"

var runnerItems = []list.Item{
	runnerItem{title: "Modal", description: "Submit a solution to be evaluated on Modal runners.", value: "modal"},
	runnerItem{title: "Github", description: "Submit a solution to be evaluated on Github runners. This can take a little longer to spin up.", value: "github"},
}

var submissionModeItems = []list.Item{
	submissionModeItem{title: "Test", description: "Test the solution and give detailed results about passed/failed tests.", value: "test"},
	submissionModeItem{title: "Benchmark", description: "Benchmark the solution, this also runs the tests and afterwards runs the benchmark, returning detailed timing results", value: "benchmark"},
	submissionModeItem{title: "Leaderboard", description: "Submit to the leaderboard, this first runs public tests and then private tests. If both pass, the submission is evaluated and submit to the leaderboard.", value: "leaderboard"},
	submissionModeItem{title: "Private", description: "TODO", value: "private"},
	submissionModeItem{title: "Script", description: "TODO", value: "script"},
	submissionModeItem{title: "Profile", description: "TODO", value: "profile"},
}
