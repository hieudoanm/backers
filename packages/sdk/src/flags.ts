import type { HttpClient } from './http.js';

/** Rule operators supported by flag targeting. */
export type FlagOp =
  | 'eq'
  | 'neq'
  | 'in'
  | 'contains'
  | 'starts_with'
  | 'ends_with'
  | 'gt'
  | 'gte'
  | 'lt'
  | 'lte';

/** One targeting condition; all rules must match (AND). */
export interface FlagRule {
  /** Attribute looked up in the evaluation context (`user_id`, `email`, or caller-supplied context keys). */
  attribute: string;
  op: FlagOp;
  value: unknown;
}

/** A stored feature flag. */
export interface Flag {
  name: string;
  enabled: boolean;
  rules: FlagRule[];
  /** Arbitrary JSON returned to callers when the flag evaluates on. */
  payload: unknown;
  description: string;
  created_at?: string;
  updated_at?: string;
}

/** Body for `POST /api/flags` — creates or replaces a flag by name. */
export interface FlagUpsert {
  name: string;
  enabled?: boolean;
  rules?: FlagRule[];
  payload?: unknown;
  description?: string;
}

/** Body for `PATCH /api/flags/{name}` — every field optional. */
export interface FlagPatch {
  enabled?: boolean;
  rules?: FlagRule[];
  payload?: unknown;
  description?: string;
}

/** Evaluation result for one flag; `payload` is present only when on. */
export interface FlagEvaluation {
  enabled: boolean;
  payload?: unknown;
}

/**
 * Feature flags client — `/api/flags`.
 * Management (list/get/upsert/update/remove) requires an admin token;
 * `evaluate` is open and uses the client's Bearer token, if any, to expose
 * `user_id`/`email` to rules.
 */
export class FlagsClient {
  constructor(private readonly http: HttpClient) {}

  /** List all flags, or a subset when `names` is given. */
  list(names?: string[]): Promise<Flag[]> {
    const query = names?.length
      ? `?names=${encodeURIComponent(names.join(','))}`
      : '';
    return this.http.get<Flag[]>(`/api/flags${query}`);
  }

  get(name: string): Promise<Flag> {
    return this.http.get<Flag>(`/api/flags/${encodeURIComponent(name)}`);
  }

  /** Create or replace a flag by name. */
  upsert(flag: FlagUpsert): Promise<Flag> {
    return this.http.post<Flag>('/api/flags', flag);
  }

  update(name: string, patch: FlagPatch): Promise<Flag> {
    return this.http.patch<Flag>(
      `/api/flags/${encodeURIComponent(name)}`,
      patch
    );
  }

  remove(name: string): Promise<void> {
    return this.http.delete(`/api/flags/${encodeURIComponent(name)}`);
  }

  /**
   * Evaluate flags for the current context. `names` empty evaluates every
   * flag; `context` adds attributes (e.g. `plan`, `country`) that rules can
   * match. Resolves to a name → evaluation map.
   */
  async evaluate(
    names?: string[],
    context?: Record<string, unknown>
  ): Promise<Record<string, FlagEvaluation>> {
    const res = await this.http.post<{ flags: Record<string, FlagEvaluation> }>(
      '/api/flags/evaluate',
      { flags: names ?? [], context: context ?? {} }
    );
    return res.flags;
  }
}
