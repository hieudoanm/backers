# Feature flags

Feature flags let you ship code behind switches you control at runtime — turn
features on per user, per plan, per percentage rollout, or kill them instantly
without a redeploy. Flags live in the `_flags` table and are managed through
`/api/flags` (admin) and resolved through `POST /api/flags/evaluate`.

## Anatomy of a flag

```json
{
  "name": "new-checkout",
  "enabled": true,
  "rules": [
    { "attribute": "plan", "op": "eq", "value": "pro" },
    { "attribute": "email", "op": "ends_with", "value": "@blazers.dev" }
  ],
  "payload": { "variant": "v2", "max_cart": 50 },
  "description": "New checkout flow for pro users"
}
```

- **`enabled`** — the master switch. `false` means the flag is off for everyone
  regardless of rules.
- **`rules`** — an array of targeting conditions. **Every rule must match**
  (logical AND). An empty array matches everyone.
- **`payload`** — arbitrary JSON returned to the caller when the flag evaluates
  on. Use it for variant names, config values, rollout metadata.
- **`description`** — free text for the admin dashboard / your own tooling.

## Targeting rules

Each rule has three parts:

| Field       | Meaning                                   |
| ----------- | ----------------------------------------- |
| `attribute` | A key looked up in the evaluation context |
| `op`        | Comparison operator (see below)           |
| `value`     | The value to compare against (any JSON)   |

### Attributes

The evaluation context is built per request from two sources:

1. **Auth claims** — when the caller sends `Authorization: Bearer <token>`,
   `user_id` and `email` are added automatically. These win over context keys of
   the same name.
2. **`context`** — any JSON object the client passes in the evaluate body, for
   attributes Blazers doesn't know about (`plan`, `country`, `app_version`, …).

```json
POST /api/flags/evaluate
{ "flags": ["new-checkout"], "context": { "plan": "pro", "country": "VN" } }
```

### Operators

| `op`                  | Matches when                                                             |
| --------------------- | ------------------------------------------------------------------------ |
| `eq`                  | attribute equals `value`                                                 |
| `neq`                 | attribute differs from `value` (a missing attribute counts as different) |
| `in`                  | attribute equals any element of `value` (an array)                       |
| `contains`            | string attribute contains `value`, or array attribute contains `value`   |
| `starts_with`         | string attribute starts with `value`                                     |
| `ends_with`           | string attribute ends with `value`                                       |
| `gt` `gte` `lt` `lte` | numeric comparison                                                       |

Numbers compare numerically; everything else compares as JSON values with a
string fallback, so `"42"` and `42` are interchangeable in `eq`.

## Evaluation

`POST /api/flags/evaluate` is **open** — no admin role needed, because flags are
meant to be read by end users. Send a token anyway when you want user-targeted
rules to see `user_id`/`email`.

```json
// request
{ "flags": ["new-checkout", "beta-banner"], "context": { "plan": "pro" } }

// response 200
{
  "flags": {
    "new-checkout": { "enabled": true, "payload": { "variant": "v2" } },
    "beta-banner":  { "enabled": false }
  }
}
```

- `flags` omitted or empty → **every** flag is evaluated.
- Unknown flag names are silently omitted from the result.
- `payload` is present only when the flag evaluated **on**.

A disabled flag, a failing rule, or a missing attribute all produce
`{ "enabled": false }` with no payload — callers only ever branch on `enabled`.

## Managing flags

All management endpoints require an admin token (`admin` role on `*`).

| Endpoint                   | Purpose                                                       |
| -------------------------- | ------------------------------------------------------------- |
| `GET /api/flags`           | List all flags (`?names=a,b` for a subset)                    |
| `POST /api/flags`          | Create or replace (upsert by `name`)                          |
| `GET /api/flags/{name}`    | Fetch one flag                                                |
| `PATCH /api/flags/{name}`  | Partial update (`enabled`, `rules`, `payload`, `description`) |
| `DELETE /api/flags/{name}` | Remove a flag                                                 |

Every mutation broadcasts a `_flags` record-change event on the WebSocket hub
(`collection: "_flags"`), so subscribed clients can invalidate cached flag state
immediately.

## Patterns

**Kill switch** — `enabled: false` takes a feature down everywhere instantly.

**Plan gating** —
`rules: [{"attribute": "plan", "op": "in", "value": ["pro", "enterprise"]}]`.
The client sends `plan` in `context` (ideally derived server-side from your
billing data, not trusted from the client).

**Internal rollout** —
`rules: [{"attribute": "email", "op": "ends_with", "value": "@yourcompany.com"}]`
evaluated with the caller's JWT.

**Percentage rollout** — not built in; approximate it by hashing a stable
attribute (`user_id`) in a JS [edge function](./functions.md) and passing the
bucket as context, or add a `rollout_pct` field to `payload` and evaluate in
your client.

## When to use what

- **Flags** — runtime on/off + targeting for _product_ behavior your clients
  read per request.
- **[Settings](./secrets.md)** — server-side configuration (SMTP, app name).
- **[Cache](./cache.md)** — high-churn ephemeral values, not flag semantics.
- **A collection** — when flags need CRUD UI, history, or per-flag records
  beyond the built-in fields.
