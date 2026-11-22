// Entry point for `@blazers/sdk` — re-exports the wasm-bindgen bundle in
// `pkg/` (built by `pnpm build` → wasm-pack) plus JS-side helpers.
export * from './pkg/blazers_sdk.js';

/**
 * Error type kept for API compatibility with the former TypeScript SDK.
 * Wasm methods reject promises with the error message string, so
 * `err instanceof BlazersError` is always false — use `String(err)` or
 * `err?.message ?? String(err)` in catch blocks.
 */
export class BlazersError extends Error {
  constructor(message) {
    super(message);
    this.name = 'BlazersError';
  }
}
