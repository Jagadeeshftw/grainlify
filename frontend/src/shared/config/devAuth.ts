/**
 * @file DEV-ONLY authentication bypass for local testing.
 *
 * Lets developers reach protected routes (e.g. `/dashboard`) without completing
 * the sign-in flow, by seeding a synthetic contributor in the AuthProvider.
 * Intended purely for working on authenticated UI (such as the onboarding tour)
 * while the real sign-in flow is unavailable.
 *
 * @security HARD-GATED behind `import.meta.env.DEV`, which Vite statically
 * replaces with `false` in every production build. Rollup then dead-code-
 * eliminates every branch guarded by {@link AUTH_BYPASS_ENABLED}, so the mock
 * user and the bypass logic cannot exist in a production bundle. The flag also
 * requires the explicit opt-in env var `VITE_AUTH_BYPASS=true`, so it stays OFF
 * by default even during `vite dev`.
 */

/**
 * `true` only when running the dev server (`vite dev`) **and**
 * `VITE_AUTH_BYPASS` is set to `'true'`. Always `false` in production builds.
 */
export const AUTH_BYPASS_ENABLED: boolean =
  import.meta.env.DEV && import.meta.env.VITE_AUTH_BYPASS === 'true';
