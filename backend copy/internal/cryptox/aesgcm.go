package cryptox

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"encoding/base64"
	"fmt"
	"io"
)

func KeyFromB64(b64 string) ([]byte, error) {
	if b64 == "" {
		return nil, fmt.Errorf("TOKEN_ENC_KEY_B64 is required")
	}
	key, err := base64.StdEncoding.DecodeString(b64)
	if err != nil {
		return nil, fmt.Errorf("decode TOKEN_ENC_KEY_B64: %w", err)
	}
	if len(key) != 32 {
		return nil, fmt.Errorf("TOKEN_ENC_KEY_B64 must decode to 32 bytes")
	}
	return key, nil
}

// EncryptAESGCM returns nonce||ciphertext (ciphertext includes GCM tag).
func EncryptAESGCM(key []byte, plaintext []byte) ([]byte, error) {
	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}
	nonce := make([]byte, gcm.NonceSize())
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return nil, err
	}
	ct := gcm.Seal(nil, nonce, plaintext, nil)
	out := make([]byte, 0, len(nonce)+len(ct))
	out = append(out, nonce...)
	out = append(out, ct...)
	return out, nil
}

func DecryptAESGCM(key []byte, blob []byte) ([]byte, error) {
	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}
	if len(blob) < gcm.NonceSize() {
		return nil, fmt.Errorf("ciphertext too short")
	}
	nonce := blob[:gcm.NonceSize()]
	ct := blob[gcm.NonceSize():]
	return gcm.Open(nil, nonce, ct, nil)
}





















