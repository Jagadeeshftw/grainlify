# Glassmorphism Landing Page

This is a code bundle for Glassmorphism Landing Page. The original project is available at https://www.figma.com/design/Q7mcDMFYoct92SkOWFAoCP/Glassmorphism-Landing-Page.

## Prerequisites

- **Node.js**: Latest LTS version (recommended: v20.x or higher)
- **pnpm**: Package manager (install globally with `npm install -g pnpm` if not already installed)

## Setup

1. **Install dependencies**:
   ```bash
   pnpm install
   ```

2. **Configure environment variables**:
   ```bash
   cp .env.example .env
   ```
   
   Then edit `.env` and set the required variables:
   - `VITE_API_BASE_URL`: Backend API URL (e.g., `http://localhost:8080`)
   - `VITE_FRONTEND_BASE_URL`: Frontend base URL (optional, defaults to current origin)

## Running the code

Run `pnpm run dev` to start the development server.
