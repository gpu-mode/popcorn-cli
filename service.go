package main

import (
	"encoding/json"
	"io"
	"net/http"

	"github.com/charmbracelet/bubbles/list"
)

func fetchLeaderboards() ([]leaderboardItem, error) {
	resp, err := http.Get(BASE_URL + "/leaderboards")
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	var leaderboards []map[string]interface{}
	err = json.Unmarshal(body, &leaderboards)
	if err != nil {
		return nil, err
	}

	leaderboardNames := make([]leaderboardItem, len(leaderboards))
	for i, lb := range leaderboards {
		leaderboardNames[i] = leaderboardItem{title: lb["name"].(string)}
	}

	return leaderboardNames, nil
}

func fetchAvailableGpus(leaderboard string, runner string) ([]gpuItem, error) {
	resp, err := http.Get(BASE_URL + "/" + leaderboard + "/" + runner + "/gpus")
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	var gpus []string
	err = json.Unmarshal(body, &gpus)
	if err != nil {
		return nil, err
	}

	gpuItems := make([]gpuItem, len(gpus))
	for i, gpu := range gpus {
		gpuItems[i] = gpuItem{title: gpu}
	}

	return gpuItems, nil
}

func getListItems[T list.Item](fetchFn func() ([]T, error)) ([]list.Item, error) {
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
