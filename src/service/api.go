package service

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

	"popcorn-cli/src/models"
)

var BASE_URL = os.Getenv("POPCORN_API_URL")

func FetchLeaderboards() ([]models.LeaderboardItem, error) {
	resp, err := http.Get(BASE_URL + "/leaderboards")
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("failed to fetch leaderboards: %s", resp.Status)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	var leaderboards []map[string]interface{}
	err = json.Unmarshal(body, &leaderboards)
	if err != nil {
		return nil, err
	}

	leaderboardNames := make([]models.LeaderboardItem, len(leaderboards))
	for i, lb := range leaderboards {
		leaderboardNames[i] = models.LeaderboardItem{TitleText: lb["name"].(string)}
	}

	return leaderboardNames, nil
}

func FetchAvailableGpus(leaderboard string, runner string) ([]models.GpuItem, error) {
	resp, err := http.Get(BASE_URL + "/" + leaderboard + "/" + runner + "/gpus")
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("failed to fetch GPUs: %s", resp.Status)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	var gpus []string
	err = json.Unmarshal(body, &gpus)
	if err != nil {
		return nil, err
	}

	gpuItems := make([]models.GpuItem, len(gpus))
	for i, gpu := range gpus {
		gpuItems[i] = models.GpuItem{TitleText: gpu}
	}

	return gpuItems, nil
}

func SubmitSolution(leaderboard string, runner string, gpu string, submissionMode string, filename string, fileContent []byte) (string, error) {
	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)

	part, err := writer.CreateFormFile("file", filepath.Base(filename))
	if err != nil {
		return "", fmt.Errorf("error creating form file: %s", err)
	}

	if _, err := part.Write(fileContent); err != nil {
		return "", fmt.Errorf("error writing file to form: %s", err)
	}

	if err := writer.Close(); err != nil {
		return "", fmt.Errorf("error closing form: %s", err)
	}

	url := fmt.Sprintf("%s/%s/%s/%s/%s",
		BASE_URL,
		strings.ToLower(leaderboard),
		strings.ToLower(runner),
		strings.ToLower(gpu),
		strings.ToLower(submissionMode))

	req, err := http.NewRequest("POST", url, body)
	if err != nil {
		return "", fmt.Errorf("error creating request: %s", err)
	}

	req.Header.Set("Content-Type", writer.FormDataContentType())

	client := &http.Client{Timeout: 60 * time.Second}

	resp, err := client.Do(req)
	if err != nil {
		return "", fmt.Errorf("error sending request: %s", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("error reading response body: %s", err)
	}

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("server returned status %d: %s", resp.StatusCode, string(respBody))
	}

	var result struct {
		Status string         `json:"status"`
		Result map[string]any `json:"result"`
	}
	if err := json.Unmarshal(respBody, &result); err != nil {
		return "", fmt.Errorf("error unmarshalling response body: %s", err)
	}

	prettyResult, err := json.MarshalIndent(result.Result, "", "  ")
	if err != nil {
		return "", fmt.Errorf("error marshalling response body: %s", err)
	}

	return string(prettyResult), nil
}

func GetListItems[T list.Item](fetchFn func() ([]T, error)) ([]list.Item, error) {
	items, err := fetchFn()
	if err != nil {
		return nil, err
	}

	listItems := make([]list.Item, len(items))
	for i, item := range items {
		listItems[i] = list.Item(item)
	}

	return listItems, nil
}

