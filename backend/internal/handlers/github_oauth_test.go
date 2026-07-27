package handlers

import (
	"testing"

	"github.com/jagadeesh/grainlify/backend/internal/config"
)

func TestIsAllowedRedirectURI(t *testing.T) {
	cfg := config.Config{
		CORSOrigins:     "https://app.grainlify.com, https://dashboard.grainlify.com",
		FrontendBaseURL: "https://grainlify.com",
	}

	tests := []struct {
		name        string
		redirectURI string
		cfg         config.Config
		want        bool
	}{
		{
			name:        "valid localhost origin with port",
			redirectURI: "http://localhost:3000/auth/callback",
			cfg:         cfg,
			want:        true,
		},
		{
			name:        "valid 127.0.0.1 origin",
			redirectURI: "http://127.0.0.1:5173",
			cfg:         cfg,
			want:        true,
		},
		{
			name:        "valid https localhost",
			redirectURI: "https://localhost:8443/cb",
			cfg:         cfg,
			want:        true,
		},
		{
			name:        "valid vercel preview domain",
			redirectURI: "https://pr-123.vercel.app/auth/callback",
			cfg:         cfg,
			want:        true,
		},
		{
			name:        "valid vercel apex app domain",
			redirectURI: "https://my-app.vercel.app",
			cfg:         cfg,
			want:        true,
		},
		{
			name:        "explicit allowed CORS origin",
			redirectURI: "https://app.grainlify.com/auth/callback",
			cfg:         cfg,
			want:        true,
		},
		{
			name:        "explicit FrontendBaseURL origin",
			redirectURI: "https://grainlify.com/auth/callback",
			cfg:         cfg,
			want:        true,
		},
		{
			name:        "reject host spoofing localhost attacker domain",
			redirectURI: "http://localhost.attacker.com",
			cfg:         cfg,
			want:        false,
		},
		{
			name:        "reject host spoofing vercel app attacker domain",
			redirectURI: "https://vercel.app.attacker.com",
			cfg:         cfg,
			want:        false,
		},
		{
			name:        "reject javascript scheme",
			redirectURI: "javascript:alert(1)",
			cfg:         cfg,
			want:        false,
		},
		{
			name:        "reject data scheme",
			redirectURI: "data:text/html,<script>alert(1)</script>",
			cfg:         cfg,
			want:        false,
		},
		{
			name:        "reject completely untrusted domain",
			redirectURI: "https://malicious-site.example.com",
			cfg:         cfg,
			want:        false,
		},
		{
			name:        "reject empty string",
			redirectURI: "",
			cfg:         cfg,
			want:        false,
		},
		{
			name:        "reject malformed URL",
			redirectURI: "ht tp://invalid-url",
			cfg:         cfg,
			want:        false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := isAllowedRedirectURI(tt.redirectURI, tt.cfg)
			if got != tt.want {
				t.Errorf("isAllowedRedirectURI(%q) = %v; want %v", tt.redirectURI, got, tt.want)
			}
		})
	}
}

func TestEncodeAndDecodeStateWithRedirect(t *testing.T) {
	t.Run("happy path with redirect URI", func(t *testing.T) {
		csrfToken := "random_csrf_token_32_bytes_length"
		redirectURI := "https://preview.vercel.app/callback"

		encoded := encodeStateWithRedirect(csrfToken, redirectURI)
		if encoded == csrfToken {
			t.Errorf("expected encoded state to differ from raw csrf token, got %q", encoded)
		}

		gotToken, gotRedirect, err := decodeStateWithRedirect(encoded)
		if err != nil {
			t.Fatalf("unexpected error decoding state: %v", err)
		}
		if gotToken != csrfToken {
			t.Errorf("got token %q; want %q", gotToken, csrfToken)
		}
		if gotRedirect != redirectURI {
			t.Errorf("got redirect %q; want %q", gotRedirect, redirectURI)
		}
	})

	t.Run("backward compatible empty redirect URI", func(t *testing.T) {
		csrfToken := "legacy_csrf_token_only"

		encoded := encodeStateWithRedirect(csrfToken, "")
		if encoded != csrfToken {
			t.Errorf("expected encodeStateWithRedirect with empty redirect to return raw token, got %q", encoded)
		}

		gotToken, gotRedirect, err := decodeStateWithRedirect(csrfToken)
		if err != nil {
			t.Fatalf("unexpected error decoding state: %v", err)
		}
		if gotToken != csrfToken {
			t.Errorf("got token %q; want %q", gotToken, csrfToken)
		}
		if gotRedirect != "" {
			t.Errorf("got redirect %q; want empty string", gotRedirect)
		}
	})

	t.Run("malformed or unencoded base64 state fallback", func(t *testing.T) {
		malformedState := "not_a_valid_base64_encoded_string_!!!"

		gotToken, gotRedirect, err := decodeStateWithRedirect(malformedState)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if gotToken != malformedState {
			t.Errorf("expected fallback token to be %q, got %q", malformedState, gotToken)
		}
		if gotRedirect != "" {
			t.Errorf("expected redirect to be empty, got %q", gotRedirect)
		}
	})

	t.Run("state with multiple pipe characters", func(t *testing.T) {
		csrfToken := "token_with_pipes"
		redirectURI := "https://example.com/path?param1=a|b"

		encoded := encodeStateWithRedirect(csrfToken, redirectURI)
		gotToken, gotRedirect, err := decodeStateWithRedirect(encoded)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if gotToken != csrfToken {
			t.Errorf("got token %q; want %q", gotToken, csrfToken)
		}
		if gotRedirect != redirectURI {
			t.Errorf("got redirect %q; want %q", gotRedirect, redirectURI)
		}
	})
}

func TestDeterminismAcrossRetries(t *testing.T) {
	cfg := config.Config{
		CORSOrigins: "https://app.grainlify.com",
	}
	csrfToken := "deterministic_csrf_token_001"
	redirectURI := "https://app.grainlify.com/callback"

	initialEncoded := encodeStateWithRedirect(csrfToken, redirectURI)
	initialToken, initialRedirect, initialErr := decodeStateWithRedirect(initialEncoded)
	initialAllowed := isAllowedRedirectURI(redirectURI, cfg)

	if initialErr != nil {
		t.Fatalf("unexpected error in initial decode: %v", initialErr)
	}

	for i := 0; i < 100; i++ {
		encoded := encodeStateWithRedirect(csrfToken, redirectURI)
		if encoded != initialEncoded {
			t.Fatalf("retry %d: encoded state mismatch: got %q; want %q", i, encoded, initialEncoded)
		}

		tok, red, err := decodeStateWithRedirect(encoded)
		if err != nil {
			t.Fatalf("retry %d: unexpected error decoding: %v", i, err)
		}
		if tok != initialToken || red != initialRedirect {
			t.Fatalf("retry %d: decoded output mismatch: got (%q, %q); want (%q, %q)", i, tok, red, initialToken, initialRedirect)
		}

		allowed := isAllowedRedirectURI(redirectURI, cfg)
		if allowed != initialAllowed {
			t.Fatalf("retry %d: isAllowedRedirectURI mismatch: got %v; want %v", i, allowed, initialAllowed)
		}
	}
}
