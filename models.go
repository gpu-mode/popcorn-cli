package main

type leaderboardItem struct {
	title string
}

func (i leaderboardItem) FilterValue() string { return i.title }
func (i leaderboardItem) Title() string       { return i.title }
func (i leaderboardItem) Description() string { return "" }

type gpuItem struct {
	title string
}

func (i gpuItem) FilterValue() string { return i.title }
func (i gpuItem) Title() string       { return i.title }
func (i gpuItem) Description() string { return "" }

type runnerItem struct {
	title       string
	description string
	value       string
}

func (i runnerItem) FilterValue() string { return i.title }
func (i runnerItem) Title() string       { return i.title }
func (i runnerItem) Description() string { return i.description }

type submissionModeItem struct {
	title       string
	description string
}

func (i submissionModeItem) FilterValue() string { return i.title }
func (i submissionModeItem) Title() string       { return i.title }
func (i submissionModeItem) Description() string { return i.description }

type modelState int

const (
	modelStateLeaderboardSelection modelState = iota
	modelStateRunnerSelection
	modelStateGpuSelection
	modelStateSubmissionModeSelection
	modelStateWaitingForResult
)

type errorMsg struct {
	err error
}

type submissionResultMsg string
