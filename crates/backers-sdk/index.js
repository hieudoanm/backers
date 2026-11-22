// Entry point for `@backers/sdk` — re-exports the wasm-bindgen bundle in
// `pkg/` (built by `pnpm build` → wasm-pack) plus JS-side helpers.
export * from './pkg/backers_sdk.js';

/**
 * Error type kept for API compatibility with the former TypeScript SDK.
 * Wasm methods reject promises with the error message string, so
 * `err instanceof BackersError` is always false — use `String(err)` or
 * `err?.message ?? String(err)` in catch blocks.
 */
export class BackersError extends Error {
  constructor(message) {
    super(message);
    this.name = 'BackersError';
  }
}
