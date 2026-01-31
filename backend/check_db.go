package main

import (
	"context"
	"fmt"
	"log"
	"os"

	"github.com/jackc/pgx/v4/pgxpool"
	"github.com/joho/godotenv"
)

func main() {
	_ = godotenv.Load(".env")
	dbURL := os.Getenv("DATABASE_URL")
	if dbURL == "" {
		dbURL = "postgres://postgres:postgres@localhost:5432/patchwork?sslmode=disable"
	}

	pool, err := pgxpool.Connect(context.Background(), dbURL)
	if err != nil {
		log.Fatal(err)
	}
	defer pool.Close()

	rows, err := pool.Query(context.Background(), "SELECT id, login, first_name, last_name FROM users LIMIT 10")
	if err != nil {
		log.Fatal(err)
	}
	defer rows.Close()

	fmt.Println("--- Users in Database ---")
	for rows.Next() {
		var id, login string
		var firstName, lastName *string
		err := rows.Scan(&id, &login, &firstName, &lastName)
		if err != nil {
			log.Fatal(err)
		}
		fn := "nil"
		if firstName != nil {
			fn = *firstName
		}
		ln := "nil"
		if lastName != nil {
			ln = *lastName
		}
		fmt.Printf("ID: %s, Login: %s, First: %s, Last: %s\n", id, login, fn, ln)
	}
}
