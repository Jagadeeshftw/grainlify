# syntax=docker/dockerfile:1

FROM golang:1.24-alpine AS build

WORKDIR /src

# Dependencies for module download (git) and CA certs for HTTPS module fetches.
RUN apk add --no-cache git ca-certificates

# Copy only go.{mod,sum} first to leverage Docker layer caching.
COPY backend/go.mod backend/go.sum ./backend/

WORKDIR /src/backend
RUN go mod download

# Copy the backend source and build the API binary.
COPY backend/ ./

RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 \
  go build -trimpath -ldflags="-s -w" -o /out/api ./cmd/api

FROM alpine:3.20

RUN apk add --no-cache ca-certificates tzdata \
  && adduser -D -H -u 10001 appuser

WORKDIR /app
COPY --from=build /out/api /app/api

USER appuser

ENV PORT=8080
EXPOSE 8080

CMD ["/app/api"]


