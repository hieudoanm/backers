/**
 * Types for `@blazers/sdk` — the Rust SDK compiled to WASM.
 *
 * The runtime classes are the `#[wasm_bindgen]` exports generated into
 * `pkg/` by `wasm-pack build`; this file declares the public TypeScript
 * surface (class shapes + wire/data types) so consumers get real types
 * instead of `Promise<any>`.
 */

export declare class BlazersError extends Error {}

export interface ClientOptions {
  /** Base URL of the Blazers server, e.g. `http://localhost:8090`. */
  url: string;
  /** Bearer token attached to every request (set automatically on login). */
  token?: string;
  /** Extra headers sent with every request. */
  headers?: Record<string, string>;
}

export interface Credentials {
  email: string;
  password: string;
}

export interface UpdateMeOptions {
  data?: Record<string, unknown>;
  /** New password (also links anonymous accounts). */
  password?: string;
  /** Required when the account already has a password. */
  currentPassword?: string;
  /** New email — links the account when set on an anonymous user. */
  email?: string;
}

export interface MfaSetup {
  secret: string;
  otpauth_url: string;
}

export interface User {
  id: string;
  email: string;
  /** Custom user fields (schemaless JSON). */
  data: Record<string, unknown>;
  verified: boolean;
  mfa_enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface LoginResponse {
  user: User;
  token: string;
}

/** `POST /api/auth/login` returns this union: full session or an MFA challenge. */
export type LoginResult =
  | ({ mfa_required?: false } & LoginResponse)
  | { mfa_required: true; user: User };

export interface RecordData {
  id: string;
  /** Schemaless document; `expand_<field>` keys appear here after `?expand=`. */
  data: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface RecordsPage {
  records: RecordData[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

export interface ListOptions {
  page?: number;
  perPage?: number;
  /** Free-text search across record data. */
  search?: string;
  /** PocketBase-style filter expression, e.g. `status="live" && views>10`. */
  filter?: string;
  /** Comma-separated fields; prefix with `-` for descending. */
  sort?: string;
  /** Comma-separated relation fields to resolve into `data.expand_<field>`. */
  expand?: string;
}

export interface CollectionRules {
  list_rule?: string;
  view_rule?: string;
  create_rule?: string;
  update_rule?: string;
  delete_rule?: string;
}

export interface Collection {
  name: string;
  /** JSON-schema string validating `record.data`. */
  schema: string;
  list_rule: string;
  view_rule: string;
  create_rule: string;
  update_rule: string;
  delete_rule: string;
  /** "records" for data collections, "auth" for the managed users collection. */
  type: 'records' | 'auth' | string;
  created_at: string;
  updated_at: string;
}

export interface Bucket {
  name: string;
  is_public: boolean;
  created_at: string;
  updated_at: string;
}

export interface FileRecord {
  id: string;
  bucket: string;
  filename: string;
  mime_type: string;
  size: number;
  created_at: string;
  updated_at: string;
}

export interface FilesPage {
  files: FileRecord[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

export interface BatchSubRequest {
  method: string;
  /** Path or absolute URL under the same origin. */
  url: string;
  headers?: Record<string, string>;
  body?: unknown;
}

export interface BatchResult {
  status: number;
  body: unknown;
}

export interface FileToken {
  token: string;
  expires_in: number;
}

export interface EdgeFunction {
  name: string;
  code: string;
  created_at?: string;
  updated_at?: string;
}

// ---- Feature flags ----

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

// ---- Realtime (WebSocket `/ws`) ----

export type RecordAction = 'create' | 'update' | 'delete';

export interface RecordEvent {
  type: 'record';
  action: RecordAction;
  collection: string;
  record: RecordData;
  /** Present on update/delete: the record as it was before the mutation. */
  oldRecord?: RecordData;
}

export type PresenceAction = 'join' | 'leave' | 'state';

export interface PresenceEvent {
  type: 'presence';
  action: PresenceAction;
  channel: string;
  /** Client id of the member that changed. */
  client: string;
  state?: unknown;
}

/** Any other frame the server pushes (manual broadcasts, pings, …). */
export interface RawEvent {
  type: string;
  [key: string]: unknown;
}

export type RealtimeEvent = RecordEvent | PresenceEvent | RawEvent;

// ---- Client classes (wasm-bindgen exports in pkg/) ----

export declare class Blazers {
  /** `new Blazers({ url, token? })` or `new Blazers("http://…")`. */
  constructor(opts: ClientOptions | string);
  readonly auth: AuthJs;
  readonly collections: CollectionsJs;
  readonly files: FilesJs;
  readonly flags: FlagsJs;
  readonly functions: FunctionsJs;
  readonly realtime: RealtimeJs;
  readonly http: HttpJs;
  /** Client for one collection's records. */
  collection(name: string): CollectionJs;
  /** `GET /api/health`. */
  health(): Promise<unknown>;
}

export declare class HttpJs {
  /** Sets the bearer token sent on every request (`undefined`/`null` clears). */
  setToken(token?: string | null): void;
  /** Current bearer token, or `undefined`. */
  readonly authToken: string | undefined;
  /** Base URL the client was configured with. */
  readonly baseUrl: string;
}

export declare class AuthJs {
  register(opts: Credentials): Promise<User>;
  /** Returns a full session or an MFA challenge (`mfa_required: true`). */
  login(opts: Credentials): Promise<LoginResult>;
  /** Completes an MFA challenge. */
  mfaLogin(opts: {
    email: string;
    password: string;
    code: string;
  }): Promise<LoginResponse>;
  refresh(): Promise<LoginResponse>;
  logout(): void;
  me(): Promise<User>;
  updateMe(opts?: UpdateMeOptions): Promise<User>;
  /** Response may include `dev_token` when SMTP isn't configured. */
  forgotPassword(
    email: string
  ): Promise<{ message?: string; dev_token?: string }>;
  resetPassword(token: string, password: string): Promise<unknown>;
  /** Response may include `dev_token` when SMTP isn't configured. */
  requestVerification(
    email: string
  ): Promise<{ message?: string; dev_token?: string }>;
  confirmVerification(token: string): Promise<unknown>;
  /** Response may include `dev_otp` when SMTP isn't configured. */
  requestOtp(email: string): Promise<{ message?: string; dev_otp?: string }>;
  /** Passwordless login via emailed OTP — stores the session token. */
  authWithOtp(email: string, otp: string): Promise<LoginResponse>;
  signInAnonymously(): Promise<LoginResponse>;
  oauthUrl(provider: string): string;
}

export declare class CollectionJs {
  list(opts?: ListOptions): Promise<RecordsPage>;
  listAll(opts?: ListOptions, maxPages?: number): Promise<RecordData[]>;
  get(id: string): Promise<RecordData>;
  create(data: Record<string, unknown>, id?: string): Promise<RecordData>;
  update(id: string, data: Record<string, unknown>): Promise<RecordData>;
  delete(id: string): Promise<void>;
}

export declare class CollectionsJs {
  list(): Promise<Collection[]>;
  get(name: string): Promise<Collection>;
  /** `schema` may be a JSON string or a plain object. */
  create(name: string, schema?: unknown): Promise<Collection>;
  update(
    name: string,
    opts?: {
      schema?: unknown;
      listRule?: string;
      viewRule?: string;
      createRule?: string;
      updateRule?: string;
      deleteRule?: string;
    }
  ): Promise<Collection>;
  delete(name: string): Promise<void>;
}

export declare class FilesJs {
  listBuckets(): Promise<Bucket[]>;
  createBucket(name: string, isPublic: boolean): Promise<Bucket>;
  getBucket(name: string): Promise<Bucket>;
  deleteBucket(name: string): Promise<void>;
  listFiles(
    bucket: string,
    page?: number,
    perPage?: number
  ): Promise<FilesPage>;
  /** Uploads a `Blob`/`File` (browser) — `file.name`/`file.type` are used. */
  upload(bucket: string, file: Blob): Promise<FileRecord>;
  /** Uploads raw bytes (`Uint8Array`). */
  uploadBytes(
    bucket: string,
    filename: string,
    data: Uint8Array,
    contentType?: string
  ): Promise<FileRecord>;
  downloadUrl(bucket: string, fileId: string, token?: string): string;
  download(bucket: string, fileId: string, token?: string): Promise<Uint8Array>;
  fileToken(
    bucket: string,
    fileId: string,
    opts?: { collection?: string; record?: string }
  ): Promise<FileToken>;
  deleteFile(bucket: string, fileId: string): Promise<void>;
}

export declare class FlagsJs {
  list(names?: string[]): Promise<Flag[]>;
  get(name: string): Promise<Flag>;
  upsert(opts: FlagUpsert): Promise<Flag>;
  update(name: string, opts: FlagPatch): Promise<Flag>;
  remove(name: string): Promise<void>;
  evaluate(
    names?: string[],
    context?: Record<string, unknown>
  ): Promise<Record<string, FlagEvaluation>>;
}

export declare class FunctionsJs {
  list(): Promise<EdgeFunction[]>;
  upsert(name: string, code: string): Promise<EdgeFunction>;
  remove(name: string): Promise<void>;
  invoke(name: string, input?: unknown): Promise<unknown>;
}

export declare class RealtimeJs {
  connect(): Promise<void>;
  readonly connected: boolean;
  subscribe(collection: string, opts?: { filter?: string }): Promise<void>;
  unsubscribe(collection: string): Promise<void>;
  presenceJoin(channel: string, opts?: { state?: unknown }): Promise<void>;
  presenceUpdate(channel: string, state: unknown): Promise<void>;
  presenceLeave(channel: string): Promise<void>;
  /** Registers a listener for every realtime frame; returns unsubscribe. */
  onMessage(cb: (event: RealtimeEvent) => void): () => void;
  close(): void;
}
