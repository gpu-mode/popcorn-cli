package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/list"
	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	lipgloss "github.com/charmbracelet/lipgloss"
)

var docStyle = lipgloss.NewStyle().Margin(1, 2)
var p *tea.Program

type model struct {
	filepath               string
	leaderboardsList       list.Model
	selectedLeaderboard    string
	runnersList            list.Model
	selectedRunner         string
	gpusList               list.Model
	selectedGpu            string
	submissionModeList     list.Model
	selectedSubmissionMode string
	modalState             modelState
	width                  int
	height                 int
	submissionResult       string

	finalStatus            string
	finishedOkay           bool

	spinner spinner.Model
}

func (m model) Init() tea.Cmd {
	return tea.EnterAltScreen
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd

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
			case modelStateLeaderboardSelection:
				if i := m.leaderboardsList.SelectedItem(); i != nil {
					m.selectedLeaderboard = i.(leaderboardItem).title
					m.modalState = modelStateRunnerSelection
					m.runnersList.SetSize(m.width-2, m.height-2)
				}
			case modelStateRunnerSelection:
				if i := m.runnersList.SelectedItem(); i != nil {
					m.selectedRunner = i.(runnerItem).value
					m.modalState = modelStateGpuSelection
					gpus, err := getListItems(func() ([]gpuItem, error) {
						return fetchAvailableGpus(m.selectedLeaderboard, m.selectedRunner)
					})
					if err != nil {
						m.SetError(fmt.Sprintf("Error fetching GPUs: %s", err))
						return m, tea.Quit
					}
					if len(gpus) == 0 {
						m.SetError("No GPUs available for this runner and leaderboard.")
						return m, tea.Quit
					}
					m.gpusList = list.New(gpus, list.NewDefaultDelegate(), m.width-2, m.height-2)
				}
			case modelStateGpuSelection:
				if i := m.gpusList.SelectedItem(); i != nil {
					m.selectedGpu = i.(gpuItem).title
					m.modalState = modelStateSubmissionModeSelection
					m.submissionModeList.SetSize(m.width-2, m.height-2)
				}
			case modelStateSubmissionModeSelection:
				if i := m.submissionModeList.SelectedItem(); i != nil {
					m.selectedSubmissionMode = i.(submissionModeItem).value
					m.modalState = modelStateWaitingForResult
					return m, m.Submit()
				}
			case modelStateWaitingForResult:
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
		case modelStateLeaderboardSelection:
			m.leaderboardsList.SetSize(listWidth, listHeight)
		case modelStateRunnerSelection:
			m.runnersList.SetSize(listWidth, listHeight)
		case modelStateGpuSelection:
			m.gpusList.SetSize(listWidth, listHeight)
		case modelStateSubmissionModeSelection:
			m.submissionModeList.SetSize(listWidth, listHeight)
		}
	}

	switch m.modalState {
	case modelStateLeaderboardSelection:
		m.leaderboardsList, cmd = m.leaderboardsList.Update(msg)
	case modelStateRunnerSelection:
		m.runnersList, cmd = m.runnersList.Update(msg)
	case modelStateGpuSelection:
		m.gpusList, cmd = m.gpusList.Update(msg)
	case modelStateSubmissionModeSelection:
		m.submissionModeList, cmd = m.submissionModeList.Update(msg)
	case modelStateWaitingForResult:
		m.spinner, cmd = m.spinner.Update(msg)
	}

	switch msg := msg.(type) {
	case errorMsg:
		m.SetError(msg.err.Error())
		return m, nil
	case submissionResultMsg:
		m.finalStatus = string(msg)
		m.finishedOkay = true
		return m, tea.Quit
	}

	return m, cmd
}

func (m model) View() string {
	var content string
	switch m.modalState {
	case modelStateLeaderboardSelection:
		content = m.leaderboardsList.View()
	case modelStateRunnerSelection:
		content = m.runnersList.View()
	case modelStateGpuSelection:
		content = m.gpusList.View()
	case modelStateSubmissionModeSelection:
		content = m.submissionModeList.View()
	case modelStateWaitingForResult:
		str := fmt.Sprintf("\n\n   %s Submitting solution...press ctrl+c to quit\n\n", m.spinner.View())
		content = str
	}
	return docStyle.Render(content)
}

func (m model) SetError(err string) {
	m.finalStatus = err
	m.finishedOkay = false
}

func (m model) Submit() tea.Cmd {
	return func() tea.Msg {
		go func() {
			fileContent, err := os.ReadFile(m.filepath)
			if err != nil {
				m.SetError(fmt.Sprintf("Error reading file: %s", err))
				return
			}

			body := &bytes.Buffer{}
			writer := multipart.NewWriter(body)

			part, err := writer.CreateFormFile("file", filepath.Base(m.filepath))
			if err != nil {
				m.SetError(fmt.Sprintf("Error creating form file: %s", err))
				p.Send(errorMsg{err})
				return
			}

			if _, err := part.Write(fileContent); err != nil {
				m.SetError(fmt.Sprintf("Error writing file to form: %s", err))
				p.Send(errorMsg{err})
				return
			}

			if err := writer.Close(); err != nil {
				m.SetError(fmt.Sprintf("Error closing form: %s", err))
				p.Send(errorMsg{err})
				return
			}

			url := fmt.Sprintf("%s/%s/%s/%s/%s",
				BASE_URL,
				strings.ToLower(m.selectedLeaderboard),
				strings.ToLower(m.selectedRunner),
				strings.ToLower(m.selectedGpu),
				strings.ToLower(m.selectedSubmissionMode))

			req, err := http.NewRequest("POST", url, body)
			if err != nil {
				m.SetError(fmt.Sprintf("Error creating request: %s", err))
				p.Send(errorMsg{err})
				return
			}

			req.Header.Set("Content-Type", writer.FormDataContentType())

			client := &http.Client{Timeout: 60 * time.Second}

			resp, err := client.Do(req)
			if err != nil {
				m.SetError(fmt.Sprintf("Error sending request: %s", err))
				p.Send(errorMsg{err})
				return
			}
			defer resp.Body.Close()

			respBody, err := io.ReadAll(resp.Body)
			if err != nil {
				m.SetError(fmt.Sprintf("Error reading response body: %s", err))
				p.Send(errorMsg{err})
				return
			}

			if resp.StatusCode != http.StatusOK {
				m.SetError(fmt.Sprintf("Server returned status %d: %s", resp.StatusCode, string(respBody)))
				p.Send(errorMsg{fmt.Errorf("server returned status %d: %s", resp.StatusCode, string(respBody))})
				return
			}

			var result struct {
				Status string         `json:"status"`
				Result map[string]any `json:"result"`
			}
			if err := json.Unmarshal(respBody, &result); err != nil {
				m.SetError(fmt.Sprintf("Error unmarshalling response body: %s", err))
				p.Send(errorMsg{err})
				return
			}

			prettyResult, err := json.MarshalIndent(result.Result, "", "  ")
			if err != nil {
				m.SetError(fmt.Sprintf("Error marshalling response body: %s", err))
				p.Send(errorMsg{err})
				return
			}

			p.Send(submissionResultMsg(prettyResult))
		}()

		return m.spinner.Tick()
	}
}

func main() {
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

	leaderboardItems, err := getListItems(fetchLeaderboards)
	if err != nil {
		fmt.Println("Error fetching leaderboards:", err)
		return
	}

	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = lipgloss.NewStyle().Foreground(lipgloss.Color("205"))

	m := model{
		filepath:           filepath,
		leaderboardsList:   list.New(leaderboardItems, list.NewDefaultDelegate(), 0, 0),
		runnersList:        list.New(runnerItems, list.NewDefaultDelegate(), 0, 0),
		submissionModeList: list.New(submissionModeItems, list.NewDefaultDelegate(), 0, 0),
		spinner:            s,
		modalState:         modelStateLeaderboardSelection,

		finishedOkay: true,
	}
	m.leaderboardsList.Title = "Leaderboards"
	m.runnersList.Title = "Runners"

	p = tea.NewProgram(m)
	finalModel, err := p.Run()
	if err != nil {
		fmt.Println("Error running program:", err)
		return
	}

	m, ok := finalModel.(model)
	if ok && m.finishedOkay {
		fmt.Printf("\nResult:\n\n%s\n", m.finalStatus)
	} else if ok && !m.finishedOkay {
		fmt.Printf("\nError:\n\n%s\n", m.finalStatus)
	}
}
