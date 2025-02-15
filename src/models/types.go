package models

type LeaderboardItem struct {
	TitleText     string
	TaskDescription string
}

func (i LeaderboardItem) FilterValue() string { return i.TitleText }
func (i LeaderboardItem) Title() string       { return i.TitleText }
func (i LeaderboardItem) Description() string { return i.TaskDescription }

type GpuItem struct {
	TitleText string
}

func (i GpuItem) FilterValue() string { return i.TitleText }
func (i GpuItem) Title() string       { return i.TitleText }
func (i GpuItem) Description() string { return "" }

type RunnerItem struct {
	TitleText       string
	DescriptionText string
	Value           string
}

func (i RunnerItem) FilterValue() string { return i.TitleText }
func (i RunnerItem) Title() string       { return i.TitleText }
func (i RunnerItem) Description() string { return i.DescriptionText }

type SubmissionModeItem struct {
	TitleText       string
	DescriptionText string
	Value           string
}

func (i SubmissionModeItem) FilterValue() string { return i.TitleText }
func (i SubmissionModeItem) Title() string       { return i.TitleText }
func (i SubmissionModeItem) Description() string { return i.DescriptionText }

type ModelState int

const (
	ModelStateLeaderboardSelection ModelState = iota
	ModelStateRunnerSelection
	ModelStateGpuSelection
	ModelStateSubmissionModeSelection
	ModelStateWaitingForResult
)

type ErrorMsg struct {
	Err error
}

type SubmissionResultMsg string
