package bus

import "context"

type Bus interface {
	Publish(ctx context.Context, subject string, data []byte) error
	Close()
}





















