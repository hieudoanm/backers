# Flags API

Concepts: [Feature flags](../concepts/feature-flags.md)

Base: `/api/flags`. Management endpoints require `Authorization: Bearer <token>`
with the `admin` role; evaluation is open (send a token to expose
`user_id`/`email` to rules).

## Flag object

```json
{
  "name": "new-checkout",
  "enabled": true,
  "rules": [{ "attribute": "plan", "op": "eq", "value": "pro" }],
  "payload": { "variant": "v2" },
  "description": "New checkout for pro users",
  "created_at": "2026-11-22T10:00:00+00:00",
  "updated_at": "2026-11-22T10:00:00+00:00"
}
```

`rules[].op` ∈ `eq`, `neq`, `in`, `contains`, `starts_with`, `ends_with`, `gt`,
`gte`, `lt`, `lte`. See
[concepts/feature-flags.md](../concepts/feature-flags.md#targeting-rules).

## `GET /api/flags` — list (admin)

**Query** — `names` (optional): comma-separated subset, e.g. `?names=a,b`.

**Response `200`** — array of flag objects.

## `POST /api/flags` — upsert (admin)

**Body**

```json
{
  "name": "new-checkout",
  "enabled": true,
  "rules": [{ "attribute": "plan", "op": "eq", "value": "pro" }],
  "payload": { "variant": "v2" },
  "description": "…"
}
```

Only `name` is required; `enabled` defaults `false`, `rules` `[]`, `payload`
`null`, `description` `""`. Upserts by `name`.

**Response `200`** — the stored flag. **Errors** — `400` empty name; `403` not
admin.

## `GET /api/flags/{name}` — get one (admin)

**Response `200`** — flag object. **Errors** — `403`, `404`.

## `PATCH /api/flags/{name}` — partial update (admin)

**Body** — any subset of `enabled`, `rules`, `payload`, `description`. Omitted
fields keep their current values.

**Response `200`** — updated flag. **Errors** — `403`, `404`.

## `DELETE /api/flags/{name}` — delete (admin)

**Response `204`**. **Errors** — `403`, `404`.

## `POST /api/flags/evaluate` — resolve flags

**Body**

```json
{
  "flags": ["new-checkout"],
  "context": { "plan": "pro", "country": "VN" }
}
```

- `flags` — names to evaluate; empty/omitted evaluates **all** flags.
- `context` — extra targeting attributes. `user_id`/`email` are added
  automatically when a valid Bearer token is sent, and override same-named
  context keys.

**Response `200`**

```json
{
  "flags": {
    "new-checkout": { "enabled": true, "payload": { "variant": "v2" } },
    "staff-only": { "enabled": false }
  }
}
```

Unknown names are omitted. `payload` appears only when `enabled` is `true`.

## Realtime

Flag mutations emit a WebSocket record-change event with `collection: "_flags"`
(`action`: `upsert`/`update`/`delete`) — subscribe to `_flags` like any
collection to invalidate cached evaluations.
