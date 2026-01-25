.PHONY: run dev install-air

# Install air for live reload
install-air:
	@echo "Installing air..."
	@go install github.com/air-verse/air@latest
	@echo "Air installed! Make sure ~/go/bin (or $${GOPATH}/bin) is in your PATH"

# Run with air (auto-reload on file changes)
dev:
	@if command -v air > /dev/null; then \
		air; \
	else \
		echo "Air not found. Installing..."; \
		$(MAKE) install-air; \
		echo "Please add ~/go/bin to your PATH or run: export PATH=\$$PATH:~/go/bin"; \
		echo "Then run 'make dev' again"; \
	fi

# Run without air (standard go run)
run:
	@go run ./cmd/api

# Build the binary
build:
	@go build -o ./api ./cmd/api


















