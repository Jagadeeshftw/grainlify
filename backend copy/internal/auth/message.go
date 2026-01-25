package auth

import "fmt"

func LoginMessage(nonce string) string {
	// Keep this stable; clients must sign this exact string.
	return fmt.Sprintf("Patchwork login. Nonce: %s", nonce)
}

// LegacyLoginMessage is kept temporarily for compatibility with early clients/tests.
func LegacyLoginMessage(nonce string) string {
	return fmt.Sprintf("Patchwork login\nNonce: %s", nonce)
}



