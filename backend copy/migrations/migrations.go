package migrations

import "embed"

// FS contains all migration SQL files.
//
// Note: embed patterns cannot use "..", so the embedding must live alongside the SQL files.
//go:embed *.sql
var FS embed.FS





















