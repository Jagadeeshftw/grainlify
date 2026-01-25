package auth

import (
	"crypto/ed25519"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"strings"

	"github.com/decred/dcrd/dcrec/secp256k1/v4"
	"github.com/decred/dcrd/dcrec/secp256k1/v4/ecdsa"
	"github.com/ethereum/go-ethereum/accounts"
	"github.com/ethereum/go-ethereum/common/hexutil"
	"github.com/ethereum/go-ethereum/crypto"
)

type WalletType string

const (
	WalletTypeEVM              WalletType = "evm"
	WalletTypeStellarEd25519   WalletType = "stellar_ed25519"
	WalletTypeStellarSecp256k1 WalletType = "stellar_secp256k1"
)

func NormalizeWalletType(v string) (WalletType, error) {
	switch WalletType(strings.ToLower(strings.TrimSpace(v))) {
	case WalletTypeEVM, WalletTypeStellarEd25519, WalletTypeStellarSecp256k1:
		return WalletType(strings.ToLower(strings.TrimSpace(v))), nil
	default:
		return "", fmt.Errorf("unsupported wallet_type")
	}
}

func NormalizeAddress(t WalletType, addr string) (string, error) {
	a := strings.TrimSpace(addr)
	if a == "" {
		return "", fmt.Errorf("address is required")
	}
	switch t {
	case WalletTypeEVM:
		// Normalize to 0x-prefixed lowercase.
		a = strings.ToLower(a)
		if !strings.HasPrefix(a, "0x") {
			a = "0x" + a
		}
		if len(a) != 42 {
			return "", fmt.Errorf("invalid evm address")
		}
		return a, nil
	case WalletTypeStellarEd25519, WalletTypeStellarSecp256k1:
		// For now we treat `address` as an opaque identifier (often public key hex or account-hash).
		return strings.ToLower(a), nil
	default:
		return "", fmt.Errorf("unsupported wallet_type")
	}
}

// VerifySignature verifies a wallet signature against our canonical login message.
//
// Inputs:
// - signatureHex: hex string (0x prefix optional)
// - publicKeyHex: required for Stellar; ignored for EVM
func VerifySignature(t WalletType, address string, message string, signatureHex string, publicKeyHex string) error {
	switch t {
	case WalletTypeEVM:
		return verifyEVM(address, message, signatureHex)
	case WalletTypeStellarEd25519:
		return verifyStellarEd25519(message, signatureHex, publicKeyHex)
	case WalletTypeStellarSecp256k1:
		return verifyStellarSecp256k1(message, signatureHex, publicKeyHex)
	default:
		return fmt.Errorf("unsupported wallet_type")
	}
}

func verifyEVM(expectedAddr string, message string, signatureHex string) error {
	sig, err := hexutil.Decode(signatureHex)
	if err != nil {
		return fmt.Errorf("invalid signature hex")
	}
	if len(sig) != 65 {
		return fmt.Errorf("invalid signature length")
	}
	// Transform V from {27,28} to {0,1} if necessary.
	if sig[64] >= 27 {
		sig[64] -= 27
	}

	hash := accounts.TextHash([]byte(message))
	pub, err := crypto.SigToPub(hash, sig)
	if err != nil {
		return fmt.Errorf("signature recovery failed")
	}

	recovered := strings.ToLower(crypto.PubkeyToAddress(*pub).Hex())
	if strings.ToLower(expectedAddr) != recovered {
		return fmt.Errorf("signature does not match address")
	}
	return nil
}

func verifyStellarEd25519(message string, signatureHex string, publicKeyHex string) error {
	pubKeyBytes, err := decodeHex(publicKeyHex)
	if err != nil || len(pubKeyBytes) != ed25519.PublicKeySize {
		return fmt.Errorf("invalid public_key")
	}
	sigBytes, err := decodeHex(signatureHex)
	if err != nil || len(sigBytes) != ed25519.SignatureSize {
		return fmt.Errorf("invalid signature")
	}
	if !ed25519.Verify(ed25519.PublicKey(pubKeyBytes), []byte(message), sigBytes) {
		return fmt.Errorf("invalid signature")
	}
	return nil
}

func verifyStellarSecp256k1(message string, signatureHex string, publicKeyHex string) error {
	pubKeyBytes, err := decodeHex(publicKeyHex)
	if err != nil {
		return fmt.Errorf("invalid public_key")
	}
	pubKey, err := secp256k1ParsePubKey(pubKeyBytes)
	if err != nil {
		return fmt.Errorf("invalid public_key")
	}

	sigBytes, err := decodeHex(signatureHex)
	if err != nil {
		return fmt.Errorf("invalid signature")
	}

	// Many systems verify secp256k1 signatures over a hash; we standardize on SHA-256(message).
	h := sha256.Sum256([]byte(message))

	sig, err := parseSecp256k1Signature(sigBytes)
	if err != nil {
		return fmt.Errorf("invalid signature")
	}
	if !sig.Verify(h[:], pubKey) {
		return fmt.Errorf("invalid signature")
	}
	return nil
}

func decodeHex(s string) ([]byte, error) {
	v := strings.TrimSpace(s)
	if v == "" {
		return nil, fmt.Errorf("empty")
	}
	v = strings.TrimPrefix(v, "0x")
	return hex.DecodeString(v)
}

func secp256k1ParsePubKey(b []byte) (*secp256k1.PublicKey, error) {
	// Public keys can be 33-byte compressed or 65-byte uncompressed.
	return secp256k1.ParsePubKey(b)
}

func parseSecp256k1Signature(b []byte) (*ecdsa.Signature, error) {
	// Accept both DER and compact (64-byte R||S).
	if len(b) == 64 {
		r := new(secp256k1.ModNScalar)
		s := new(secp256k1.ModNScalar)

		// 32-byte big-endian each.
		if overflow := r.SetByteSlice(b[:32]); overflow {
			return nil, fmt.Errorf("invalid r")
		}
		if overflow := s.SetByteSlice(b[32:]); overflow {
			return nil, fmt.Errorf("invalid s")
		}
		return ecdsa.NewSignature(r, s), nil
	}
	return ecdsa.ParseDERSignature(b)
}
