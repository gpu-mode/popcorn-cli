package cmd

import (
	"encoding/base64"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"

	"github.com/google/uuid"
	"gopkg.in/yaml.v3"
)

type PopcornConfig struct {
	CliID string `yaml:"cli_id"`
}

const AUTH_URL = "https://discord.com/oauth2/authorize?client_id=1357446383497511096&response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A8000%2Fauth%2Fcli&scope=identify"

func RunAuthentication() {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		fmt.Println("Error getting home directory:", err)
		return
	}

	configFile := filepath.Join(homeDir, ".popcorn.yaml")

	config := PopcornConfig{}

	if _, err := os.Stat(configFile); os.IsNotExist(err) {
		config.CliID = uuid.New().String()

		configData, err := yaml.Marshal(&config)
		if err != nil {
			fmt.Println("Error marshaling config:", err)
			return
		}

		if err := os.WriteFile(configFile, configData, 0600); err != nil {
			fmt.Println("Error writing config file:", err)
			return
		}
	} else {
		data, err := os.ReadFile(configFile)
		if err != nil {
			fmt.Println("Error reading config file:", err)
			return
		}

		if err := yaml.Unmarshal(data, &config); err != nil {
			fmt.Println("Error parsing config file:", err)
			return
		}

		if config.CliID == "" {
			config.CliID = uuid.New().String()

			configData, err := yaml.Marshal(&config)
			if err != nil {
				fmt.Println("Error marshaling config:", err)
				return
			}

			if err := os.WriteFile(configFile, configData, 0600); err != nil {
				fmt.Println("Error writing config file:", err)
				return
			}
		}
	}

	state := base64.StdEncoding.EncodeToString([]byte(config.CliID))

	authURL := fmt.Sprintf("%s&state=%s", AUTH_URL, state)

	err = openBrowser(authURL)
	if err != nil {
		fmt.Println("Error opening browser:", err)
		return
	}
}

func openBrowser(url string) error {
	var cmd *exec.Cmd

	switch runtime.GOOS {
	case "darwin":
		cmd = exec.Command("open", url)
	case "windows":
		cmd = exec.Command("rundll32", "url.dll,FileProtocolHandler", url)
	case "linux":
		cmd = exec.Command("xdg-open", url)
	default:
		return fmt.Errorf("unsupported platform: %s", runtime.GOOS)
	}

	return cmd.Start()
}
