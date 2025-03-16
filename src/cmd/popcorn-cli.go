package cmd

import (
	"fmt"
	"os"
	"strings"

	"github.com/charmbracelet/bubbles/list"
	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/lipgloss"

	"github.com/S1ro1/popcorn-cli/src/models"
	"github.com/S1ro1/popcorn-cli/src/service"

	"github.com/S1ro1/popcorn-cli/src/utils"

	tea "github.com/charmbracelet/bubbletea"
)

var submissionModeItems = []list.Item{
	models.SubmissionModeItem{TitleText: "Test", DescriptionText: "Test the solution and give detailed results about passed/failed tests.", Value: "test"},
	models.SubmissionModeItem{TitleText: "Benchmark", DescriptionText: "Benchmark the solution, this also runs the tests and afterwards runs the benchmark, returning detailed timing results", Value: "benchmark"},
	models.SubmissionModeItem{TitleText: "Leaderboard", DescriptionText: "Submit to the leaderboard, this first runs public tests and then private tests. If both pass, the submission is evaluated and submit to the leaderboard.", Value: "leaderboard"},
	models.SubmissionModeItem{TitleText: "Private", DescriptionText: "TODO", Value: "private"},
	models.SubmissionModeItem{TitleText: "Script", DescriptionText: "TODO", Value: "script"},
	models.SubmissionModeItem{TitleText: "Profile", DescriptionText: "TODO", Value: "profile"},
}

var docStyle = lipgloss.NewStyle().Margin(1, 2)
var p *tea.Program

type model struct {
	filepath               string
	leaderboardsList       list.Model
	selectedLeaderboard    string
	gpusList               list.Model
	selectedGpu            string
	submissionModeList     list.Model
	selectedSubmissionMode string
	modalState             models.ModelState
	width                  int
	height                 int

	finalStatus  string
	finishedOkay bool

	spinner spinner.Model
}

func (m model) Init() tea.Cmd {
	return tea.EnterAltScreen
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd

	if len(m.gpusList.Items()) == 0 && m.modalState == models.ModelStateGpuSelection {
		gpus, err := service.GetListItems(func() ([]models.GpuItem, error) {
			return service.FetchAvailableGpus(m.selectedLeaderboard)
		})
		if err != nil {
			m.SetError(fmt.Sprintf("Error fetching GPUs: %s", err))
			return m, tea.Quit
		}
		m.gpusList = list.New(gpus, list.NewDefaultDelegate(), m.width-2, m.height-2)
		m.gpusList.SetSize(m.width-2, m.height-2)
	}
	if !m.finishedOkay {
		return m, tea.Quit
	}

	switch msg := msg.(type) {
	case tea.KeyMsg:
		if msg.String() == "ctrl+c" {
			return m, tea.Quit
		}
		if msg.String() == "enter" {
			switch m.modalState {
			case models.ModelStateLeaderboardSelection:
				if i := m.leaderboardsList.SelectedItem(); i != nil {
					m.selectedLeaderboard = i.(models.LeaderboardItem).TitleText
					// No gpu selected in popcorn directives, fetch gpus and move to gpu selection
					if m.selectedGpu == "" {
						gpus, err := service.GetListItems(func() ([]models.GpuItem, error) {
							return service.FetchAvailableGpus(m.selectedLeaderboard)
						})
						if err != nil {
							m.SetError(fmt.Sprintf("Error fetching GPUs: %s", err))
							return m, tea.Quit
						}
						if len(gpus) == 0 {
							m.SetError("No GPUs available for this leaderboard.")
							return m, tea.Quit
						}
						m.gpusList = list.New(gpus, list.NewDefaultDelegate(), m.width-2, m.height-2)
						m.gpusList.SetSize(m.width-2, m.height-2)
						m.modalState = models.ModelStateGpuSelection
					} else {
						m.modalState = models.ModelStateSubmissionModeSelection
						m.submissionModeList.SetSize(m.width-2, m.height-2)
					}
				}
			case models.ModelStateGpuSelection:
				if i := m.gpusList.SelectedItem(); i != nil {
					m.selectedGpu = i.(models.GpuItem).TitleText
					m.modalState = models.ModelStateSubmissionModeSelection
					m.submissionModeList.SetSize(m.width-2, m.height-2)
				}
			case models.ModelStateSubmissionModeSelection:
				if i := m.submissionModeList.SelectedItem(); i != nil {
					m.selectedSubmissionMode = i.(models.SubmissionModeItem).Value
					m.modalState = models.ModelStateWaitingForResult
					return m, m.Submit()
				}
			case models.ModelStateWaitingForResult:
				return m, nil
			}
		}

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		h, v := docStyle.GetFrameSize()
		listWidth := msg.Width - h
		listHeight := msg.Height - v

		switch m.modalState {
		case models.ModelStateLeaderboardSelection:
			m.leaderboardsList.SetSize(listWidth, listHeight)
		case models.ModelStateGpuSelection:
			m.gpusList.SetSize(listWidth, listHeight)
		case models.ModelStateSubmissionModeSelection:
			m.submissionModeList.SetSize(listWidth, listHeight)
		}
	}

	switch m.modalState {
	case models.ModelStateLeaderboardSelection:
		m.leaderboardsList, cmd = m.leaderboardsList.Update(msg)
	case models.ModelStateGpuSelection:
		m.gpusList, cmd = m.gpusList.Update(msg)
	case models.ModelStateSubmissionModeSelection:
		m.submissionModeList, cmd = m.submissionModeList.Update(msg)
	case models.ModelStateWaitingForResult:
		m.spinner, cmd = m.spinner.Update(msg)
	}

	switch msg := msg.(type) {
	case models.ErrorMsg:
		m.SetError(msg.Err.Error())
		return m, nil
	case models.SubmissionResultMsg:
		m.finalStatus = string(msg)
		m.finishedOkay = true
		return m, tea.Quit
	}

	return m, cmd
}

func (m model) View() string {
	var content string
	switch m.modalState {
	case models.ModelStateLeaderboardSelection:
		content = m.leaderboardsList.View()
	case models.ModelStateGpuSelection:
		content = m.gpusList.View()
	case models.ModelStateSubmissionModeSelection:
		content = m.submissionModeList.View()
	case models.ModelStateWaitingForResult:
		str := fmt.Sprintf("\n\n   %s Submitting solution...press ctrl+c to quit\n\n", m.spinner.View())
		content = str
	}
	return docStyle.Render(content)
}

func (m *model) SetError(err string) {
	m.finalStatus = err
	m.finishedOkay = false
}

func (m model) Submit() tea.Cmd {
	return func() tea.Msg {
		go func() {
			fileContent, err := os.ReadFile(m.filepath)
			if err != nil {
				p.Send(models.ErrorMsg{Err: fmt.Errorf("error reading file: %s", err)})
				m.SetError(fmt.Sprintf("Error reading file: %s", err))
				return
			}

			prettyResult, err := service.SubmitSolution(m.selectedLeaderboard, m.selectedGpu, m.selectedSubmissionMode, m.filepath, fileContent)
			if err != nil {
				p.Send(models.ErrorMsg{Err: fmt.Errorf("error submitting solution: %s", err)})
				m.SetError(fmt.Sprintf("Error submitting solution: %s", err))
				return
			}

			p.Send(models.SubmissionResultMsg(prettyResult))
		}()

		return m.spinner.Tick()
	}
}

func Execute() {
	args := os.Args[1:]

	if len(args) == 0 {
		fmt.Println("Usage: popgorn <filepath>")
		return
	}

	filepath := args[0]
	if _, err := os.Stat(filepath); os.IsNotExist(err) {
		fmt.Println("File does not exist: ", filepath)
		return
	}

	popcornDirectives, err := utils.GetPopcornDirectives(filepath)
	if err != nil {
		fmt.Println("Error:", err)
		var input string
		fmt.Scanln(&input)
		if strings.ToLower(input) != "y" {
			return
		}
	}

	var modalState models.ModelState
	if popcornDirectives.LeaderboardName != "" && len(popcornDirectives.Gpus) > 0 {
		modalState = models.ModelStateSubmissionModeSelection
	} else if popcornDirectives.LeaderboardName != "" {
		modalState = models.ModelStateGpuSelection
	} else {
		modalState = models.ModelStateLeaderboardSelection
	}

	var selectedGpu string
	if len(popcornDirectives.Gpus) > 0 {
		selectedGpu = popcornDirectives.Gpus[0]
	}

	leaderboardItems, err := service.GetListItems(service.FetchLeaderboards)
	if err != nil {
		fmt.Println("Error fetching leaderboards:", err)

	}

	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = lipgloss.NewStyle().Foreground(lipgloss.Color("205"))

	m := model{
		filepath:            filepath,
		leaderboardsList:    list.New(leaderboardItems, list.NewDefaultDelegate(), 0, 0),
		submissionModeList:  list.New(submissionModeItems, list.NewDefaultDelegate(), 0, 0),
		gpusList:            list.New([]list.Item{}, list.NewDefaultDelegate(), 0, 0),
		spinner:             s,
		modalState:          modalState,
		finishedOkay:        true,
		finalStatus:         "",
		selectedLeaderboard: popcornDirectives.LeaderboardName,
		selectedGpu:         selectedGpu,
	}
	m.leaderboardsList.Title = "Leaderboards"

	p = tea.NewProgram(m)
	finalModel, err := p.Run()
	if err != nil {
		fmt.Println("Error running program:", err)
		return
	}

	m, ok := finalModel.(model)
	utils.DisplayAsciiArt()
	if ok && m.finishedOkay {
		fmt.Printf("\nResult:\n\n%s\n", m.finalStatus)
	} else if ok && !m.finishedOkay {
		fmt.Printf("\nError:\n\n%s\n", m.finalStatus)
	}
}
