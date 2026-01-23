package main

import (
	"context"
	"fmt"
	"os"

	"github.com/jackc/pgx/v5/pgxpool"
)

func main() {
	dbURL := "postgresql://postgres:12345@localhost:5432/grainlify?sslmode=disable"
	db, err := pgxpool.New(context.Background(), dbURL)
	if err != nil {
		fmt.Printf("Unable to connect to database: %v\n", err)
		os.Exit(1)
	}
	defer db.Close()

	var login string
	err = db.QueryRow(context.Background(), "SELECT login FROM github_accounts LIMIT 1").Scan(&login)
	if err != nil {
		fmt.Printf("Query failed: %v\n", err)
		os.Exit(1)
	}
	fmt.Println(login)
}
