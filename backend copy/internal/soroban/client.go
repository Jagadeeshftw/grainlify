package soroban

import (
	"fmt"
	"log/slog"
	"net/http"
	"time"

	"github.com/stellar/go/clients/horizonclient"
	"github.com/stellar/go/network"
)

// Client wraps Soroban RPC client and Horizon client for contract interactions
type Client struct {
	rpcURL            string
	networkPassphrase string
	horizonClient     *horizonclient.Client
	httpClient        *http.Client
	network           Network
}

// Config holds configuration for Soroban client
type Config struct {
	RPCURL           string // Soroban RPC endpoint
	NetworkPassphrase string // Network passphrase
	Network         Network // "testnet" or "mainnet"
	HTTPTimeout     time.Duration
}

// NewClient creates a new Soroban client
func NewClient(cfg Config) (*Client, error) {
	if cfg.RPCURL == "" {
		return nil, fmt.Errorf("RPC URL is required")
	}

	if cfg.NetworkPassphrase == "" {
		// Set default based on network
		if cfg.Network == NetworkMainnet {
			cfg.NetworkPassphrase = network.PublicNetworkPassphrase
		} else {
			cfg.NetworkPassphrase = network.TestNetworkPassphrase
		}
	}

	if cfg.HTTPTimeout == 0 {
		cfg.HTTPTimeout = 30 * time.Second
	}

	// Create Horizon client
	horizonURL := "https://horizon-testnet.stellar.org"
	if cfg.Network == NetworkMainnet {
		horizonURL = "https://horizon.stellar.org"
	}

	horizonClient := &horizonclient.Client{
		HorizonURL: horizonURL,
		HTTP: &http.Client{
			Timeout: cfg.HTTPTimeout,
		},
	}

	return &Client{
		rpcURL:            cfg.RPCURL,
		networkPassphrase: cfg.NetworkPassphrase,
		horizonClient:     horizonClient,
		httpClient: &http.Client{
			Timeout: cfg.HTTPTimeout,
		},
		network: cfg.Network,
	}, nil
}

// GetNetwork returns the network type
func (c *Client) GetNetwork() Network {
	return c.network
}

// GetNetworkPassphrase returns the network passphrase
func (c *Client) GetNetworkPassphrase() string {
	return c.networkPassphrase
}

// GetHorizonClient returns the Horizon client
func (c *Client) GetHorizonClient() *horizonclient.Client {
	return c.horizonClient
}

// GetRPCURL returns the RPC URL
func (c *Client) GetRPCURL() string {
	return c.rpcURL
}

// LogContractInteraction logs a contract interaction for debugging
func (c *Client) LogContractInteraction(contractID, function string, args map[string]interface{}) {
	slog.Info("contract interaction",
		"contract_id", contractID,
		"function", function,
		"network", c.network,
		"args", args,
	)
}
