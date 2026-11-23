# Pawchive API reference (reverse-engineered)

_Compiled 2026-11-23. Checked against the live site that day._

## Sources and how much to trust them

| Source | What it gave us | Trust |
|---|---|---|
| `https://pawchive.pw/api/schema` | Only a wrapper page: it is login-gated and iframes `/api/swagger_schema` | — |
| `https://pawchive.pw/api/swagger_schema` | Public SwaggerUI page with the OpenAPI 3.0.1 spec embedded inline as `var swagger_spec = {...}` | Medium: out of date in many places (see the end of this doc) |
| Frontend JS bundle (`/static/bundle/assets/global-*.js`, `dms-*.js`) | The endpoints the live site actually calls, including some the spec doesn't list | High |
| Live requests (curl, 2026-11-23) | Actual status codes and response shapes | Highest. Endpoints marked **L** below were checked. |

The backend is a fork of the Kemono ("kemonoparty") codebase. The bundle still uses names like `kemonoFetch` and `KemonoError`.

## Basics

- **Base URL:** `https://pawchive.pw/api/v1`
- **No special headers needed.** A plain GET returns `application/json`. The Kemono `Accept: text/css` trick is not required; responses were byte-identical with either header.
- **Auth:** cookie session from logging in on the website (`/account/login`). No API keys or bearer tokens. Without a session:
  - `/account/favorites` returns `401 {}`
  - `/has_pending_dms` returns `200 false`
- **Pagination:** `o` = offset, in steps of 50 (page size 50). `o=25` returns `400 {"error":"offset not multiple of 150 or too large"}`. The "150" in that message is wrong: `o=50` and `o=150` both work.
- **Search:** `q` on `/posts`. The spec says at least 3 characters, but `q=ab` works.
- **Errors:** JSON `{"error": "..."}` with a 4xx code. Some 404s have an empty `text/html` body instead (for example `/flag` when the post is not flagged).
- **Hosting:** behind Cloudflare. Some responses carry `cache-control: s-maxage=60`.

### Services

The frontend knows 12 services: `patreon`, `fanbox`, `fantia`, `afdian`, `boosty`, `gumroad`, `subscribestar`, `dlsite`, `discord`, `onlyfans`, `fansly`, `candfans`.

**Live data only has `patreon` (74,160 creators) and `fanbox` (25,417 creators)**, per `/creators` on 2026-11-23.

## Endpoints

Legend: **S** = in the swagger spec, **B** = called by the frontend bundle, **L** = verified live.

### Meta

| Method | Path | Src | Live behaviour |
|---|---|---|---|
| GET | `/app_version` | S L | Returns the literal string `custom` as `text/html`, not a commit hash. |

### Creators

| Method | Path | Src | Live behaviour |
|---|---|---|---|
| GET | `/creators` | S B L | **Every creator in one response: about 16 MB, 99,577 entries.** The frontend adds the `hide`/`only` tag filters (see below). |
| GET | `/creators/bans` | B L | Not in the spec. Returned `404 {"error":"not found"}` (feature disabled or empty). |
| GET | `/lookup/cache/{id}?service={service}` | B L | Not in the spec. The frontend calls this `bannedArtist`. Returns `404 {"error":"not found"}` for a creator that isn't banned. |
| GET | `/{service}/user/{creator_id}/profile` | S L | See below. |
| GET | `/{service}/user/{creator_id}/links` | S L | `[]` when there are no linked accounts. Items are **profile-shaped** (see `/profile`) plus `kemono_favorited` and `reimport_pending`; `relation_id` was seen non-null (e.g. `622`). |
| GET | `/{service}/user/{creator_id}/tags` | S L | `[{"tag":"Steins;Gate","post_count":3}, ...]` |
| GET | `/{service}/user/{creator_id}/announcements` | S L | `[]` when there are none. Item shape still unknown: every creator sampled had none. |
| GET | `/{service}/user/{creator_id}/fancards` | S L | `fanbox` only. Other services return `200 []`. See below. |

`/creators` item (live):
```json
{"id": "30500811", "name": "BOKABA", "public_id": "BOKABA", "service": "patreon",
 "indexed": 1785423600, "updated": 1795222800, "favorited": 3877, "ever_imported": true}
```
`indexed` and `updated` are **Unix timestamps** here. `public_id` and `ever_imported` are not in the spec.

`/profile` (live):
```json
{"id": "110390090", "name": "G-Verse", "service": "patreon",
 "indexed": "2026-06-10T21:00:00", "updated": "2026-11-23T13:00:00",
 "public_id": "GVerse159", "relation_id": null, "ever_imported": true}
```
Here the timestamps are **ISO strings with no timezone**.

`/fancards` item (live). Compared with the spec, it adds `price`, and `ext` includes the dot:
```json
{"id": 8649, "user_id": "37736420", "file_id": 8649, "price": "1000",
 "hash": "b804…6d96", "mtime": "2026-09-16T20:38:17.209157", "ctime": "...",
 "mime": "image/webp", "ext": ".webp", "added": "2026-09-16T21:00:00",
 "size": 33406, "ihash": null}
```

### Posts

| Method | Path | Src | Live behaviour |
|---|---|---|---|
| GET | `/posts` | S L | Recent posts, 50 per page. Params: `q`, `o`. **`tag`, `hide` and `only` are ignored here** (results identical to unfiltered). Tag filtering only works on the HTML page `/posts?tag=…`. Returns the **listing shape** (below). |
| GET | `/{service}/user/{creator_id}` | S L | A creator's posts, 50 per page. Returns the **full shape**. Params:<br>• `q`: full-text search; no match returns `[]`<br>• `o`: offset; a bad value returns `400 {"error":"offset not multiple of 50"}`<br>• `tag`: exact tag name, e.g. `Steins;Gate`, URL-encoded or not. Repeat it to require **all** the tags (AND). |
| GET | `/{service}/user/{creator_id}/posts` | L | Not in the spec. Same posts in the same order as the bare path. Either works. |
| GET | `/{service}/user/{creator_id}/post/{post_id}` | S L | One post in the full shape, plus `next`/`prev` (neighbouring post IDs or `null`). |
| GET | `/{service}/user/{creator_id}/post/{post_id}/revisions` | S L | Returns `404 []` when there are no revisions. Each item has a `revision_id`. The website shows one revision at `/{service}/user/{id}/post/{post_id}/revision/{rev}`. |
| GET | `/{service}/user/{creator_id}/post/{post_id}/comments` | S L | Returns `404 {"error":"Not found"}` when there are no comments, not `[]`. |
| GET | `/{service}/user/{creator_id}/post/{post_id}/flag` | S L | 404 with an empty body if the post is not flagged. 200 if it is. |
| POST | `/{service}/user/{creator_id}/post/{post_id}/flag` | S B | Flags a post for re-import. 201 on success, 409 if already flagged. Not tested, because it changes data on the site. |

**Listing shape** (`/posts`), live:
```json
{
  "id": "176121337", "user": "110390090", "service": "patreon",
  "title": "Weekly Schedule 11/23/26",
  "substring": "<p>First ~50 chars of content…",
  "published": "2026-11-23T12:00:27",
  "file": {"name": "x.png", "path": "/244/ad/<sha256>.png", "server": "https://n2.pawchive.pw"},
  "attachments": [{"name": "...", "path": "...", "server": "..."}],
  "preview_state": "scraped", "has_full": true, "origin": "import"
}
```
`file` and `attachments` entries here can carry `"preview_only": true` and have **no `server`** (the full file is not archived), so no download URL can be built for them. This shape has `substring` instead of `content`. It does not have `added`, `edited`, `embed` or `shared_file`.

**Full shape** (single post, and a creator's post list), live:
```json
{
  "id": "...", "user": "...", "service": "...", "title": "...",
  "content": "<full HTML>", "embed": {}, "shared_file": false,
  "added": "2026-11-23T13:00:00", "published": "2026-11-23T12:00:27", "edited": "2026-11-23T12:00:27",
  "file": {"name": "...", "path": "...", "server": "https://n2.pawchive.pw"},
  "attachments": [],
  "poll": null, "captions": null, "tags": null,
  "origin": "import", "preview_state": "scraped", "has_full": true, "detail_fetched": true,
  "next": "175963506", "prev": null
}
```
`next` and `prev` only appear on the single-post endpoint. None of these extra fields are in the spec: `poll`, `captions`, `tags`, `origin`, `preview_state`, `has_full`, `detail_fetched`, `server`.

**`tags` is not a JSON array.** It is `null`, or a **Postgres array literal in a string**:
```json
"tags": "{\"Anime Reactions\",Steins;Gate}"
```
Parse it yourself: strip the braces, split on commas outside quotes, and unquote. Values containing spaces are quoted; others aren't.

Comment object (live, from a post with 278 comments):
```json
{"id": "230258633", "parent_id": null, "commenter": "179129488", "commenter_name": "…",
 "content": "…", "published": "2026-11-05T14:01:01.452000",
 "revisions": [{"id": 116188, "content": "…", "added": "2026-11-08T10:00:00"}]}
```
- `commenter_name` is not in the spec.
- `published` has microseconds.
- `revisions` is usually `[]`.
- No replies (non-null `parent_id`) showed up in the ~12 posts sampled.

Revision object (live): the full post shape plus `revision_id` (an int). It lacks `origin`, `preview_state`, `has_full` and `detail_fetched`. The list is **newest first**; one post had 17 revisions.

**Tag filters on the website:** on `/posts`, `/artists` and `/artists/updated`, the frontend adds `hide=<tags>` and `only=<tags>`. It reads them from the page URL, or from the `hide_tags`/`only_tags` keys in `localStorage`. The site-wide tags are things like `nsfw`, `wip`, `commission`, `comic` (listed at `/posts/tags`). **The JSON API ignores these filters.** `/api/v1/posts` and `/api/v1/creators` return the same data with or without them.

### Files

| Method | Path | Src | Live behaviour |
|---|---|---|---|
| GET | `/search_hash/{sha256}` | S L | See below. |

Live response:
```json
{"hash": "44ad96…3577",
 "posts": [{"id": "176121337", "user": "110390090", "service": "patreon", "title": "...",
            "content": "<truncated>", "published": "...",
            "file": {"name": "...", "node": 2, "path": "/44/ad/44ad96…3577.png"},
            "attachments": []}],
 "discord_posts": null}
```
The spec's file metadata fields (`id`, `mime`, `size`, `ihash`, `mtime`…) are not returned. `discord_posts` is `null`, not `[]`.

#### File URLs

- **Building the URL:** the post page links files as `{file.server}/data{file.path}?f={url-encoded file.name}`.
  - Example: `https://n2.pawchive.pw/data/244/ad/44ad…3577.png?f=This%20week%20TAC%2011-23-26.png`
- **Path formats:**
  - Newer files: `/2<h[0:2]>/<h[2:4]>/<sha256>.<ext>`, for example hash `44ad…` → `/244/ad/…`. These come with a `server` field (`https://n2.pawchive.pw` or `https://n3.pawchive.pw`).
  - Older files: the classic Kemono path `/<h[0:2]>/<h[2:4]>/<sha256>.<ext>` with no `server` field.
- **`search_hash` is different:** it always returns the classic path plus a numeric `node` (2 → `n2`) instead of `server`.
- **Downloads are hotlink-protected.** Requesting `/data/...` directly returns **403**. In a real browser (Chromium via REA, 2026-11-23), clicking the file on the post page loaded it with **200**:
  - The browser sent **no cookies and no token**. It sent the usual browser headers, including a `Referer` pointing back at the pawchive.pw post page. So the 403 comes from a check on where the request came from, not from login or payment.
  - Response headers included `accept-ranges` (resumable downloads), `content-disposition` (the `f=` name is used as the download filename), `etag`, `last-modified`, and `access-control-allow-origin`.
  - Practical upshot: files are meant to be fetched from the website. If you script downloads, respect that and the site's terms.
- **Thumbnails:** `https://img.pawchive.pw/thumbnail/data/<classic path>`, for example `/thumbnail/data/44/ad/44ad…3577.png`. These loaded fine in the browser.
- **Creator images (website routes):** `https://pawchive.pw/icons/{service}/{creator_id}` (avatar) and `https://pawchive.pw/banners/{service}/{creator_id}` (banner).
- **Ads:** the post page also loads ad and tracking scripts from third-party domains.

### Logging in

There is no JSON login endpoint. Log in through the website form (no captcha on login; registration uses Cloudflare Turnstile):

```bash
curl -c jar.txt -b jar.txt \
  --data-urlencode "username=USER" --data-urlencode "password=PASS" \
  --data-urlencode "location=/artists" \
  https://pawchive.pw/account/login
# → 302 to /artists?logged_in=yes&role=consumer, and sets an HttpOnly cookie named `session`
# A wrong password also answers 302 (and sets a flash `session` cookie), but the Location is
# /account/login?location=/artists. Tell success from failure by the Location, not the cookie.
```

Send the `session` cookie with every account request. A new account has role `consumer`.

### Account (needs the session cookie)

| Method | Path | Src | Live behaviour |
|---|---|---|---|
| GET | `/account/favorites?type=artist` | S B L | Logged in: `200` with a JSON array (`[]` when empty). Logged out: `401 {}`. |
| GET | `/account/favorites?type=post` | S B L | Same, for favorite posts. |
| POST / DELETE | `/favorites/creator/{service}/{creator_id}` | S B L | Adds or removes a favorite creator. See below. |
| POST / DELETE | `/favorites/post/{service}/{creator_id}/{post_id}` | S B L | Adds or removes a favorite post. See below. |
| GET | `/has_pending_dms` | B L | Not in the spec. Returns `false` (logged in with no DMs, or logged out). |

**How favorites behave:**
- Every add or remove returns **`204` with an empty body**, not the 200/302 the spec lists.
- Calls are **idempotent**: adding the same favorite twice, or removing one that isn't there, still returns 204.
- **The IDs aren't checked**: favoriting a creator ID that doesn't exist also returns 204.
- Logged out, every call returns `401 {}`.

Favorite-creator item (live): the `/profile` shape plus four extra fields.
```json
{"id": "110390090", "name": "G-Verse", "service": "patreon",
 "indexed": "2026-06-10T21:00:00", "updated": "2026-11-23T13:00:00",
 "public_id": "GVerse159", "relation_id": null, "ever_imported": true,
 "kemono_favorited": 151, "reimport_pending": false,
 "faved_seq": 34694823, "last_imported": "2026-11-23T13:00:00"}
```

Favorite-post item (live): the full post shape, including `next`/`prev`, plus `faved_seq`. Its `file` uses `node` + the classic path (like `/search_hash`), not `server`; the host is `https://n{node}.pawchive.pw`.

### Importer

| Method | Path | Src | Notes |
|---|---|---|---|
| POST | `/importer/submit` | HTML form | Not in the spec. Form fields: `service`, `session_key`, `auth_id`, `x-bc`, `device_id`, `user_agent`, `save_session_key`, `save_dms`, `auto_import`. The frontend shows or requires `auth_id`/`x-bc`/`device_id` only for some services (OnlyFans, Fanbox, Discord sections). **Not tested:** it would scrape a real subscriber account with its session key. |
| GET | `/importer/logs/{import_id}` | B | Not in the spec. Returns an array of log-line strings. The frontend polls it every ~5 s until a line equals `"done"` or matches `/import failed\|invalid or expired/i`. Not tested (needs a real import ID). |

### DMs and Community chats: HTML only

Neither has a JSON API. Every candidate under `/api/v1` returns `404 {"error":"not found"}`:
- `/{service}/user/{id}/dms`
- `/dms`
- `/{service}/user/{id}/community`
- `/{service}/user/{id}/community/{channel}`

The data is only in server-rendered HTML.

**DMs:**
- `/{service}/user/{id}/dms` shows a creator's approved DMs. The profile tab reads "DMs (N)".
- `/dms?o=N` is the site-wide feed, 50 per page.
- Each DM is a `.dm-card`, with the message in `.dm-card__content` and the date in `.dm-card__added` ("Added: YYYY-MM").
- Logged-in users see `/account/review_dms` (approve or discard their own imported DMs) and `GET /api/v1/has_pending_dms`.

**Community** (Patreon's community chat, which replaced Discord on this site):
- `/{service}/user/{id}/community` returns **302** to the first channel, at `/{service}/user/{id}/community/{channel_id}`. Channel IDs are 32-character hex.
- Pages are **25 messages each**, paginated with `?o=25, 50, …`.
- The header reads "Showing 1 - 25 of N". `.chat-page__channel` tabs list each channel's name and message count.
- Messages are `.cmsg` elements:
  - `.cmsg__author`, `.cmsg__time` (relative, e.g. "10 months ago"), `.cmsg__text`
  - optional `.cmsg__image`
  - replies in `.cmsg__reply` / `.cmsg__reply-author` / `.cmsg__reply-text`
  - `.cmsg--deleted` for deleted messages
- **Leftover Discord routes:** `/api/v1/discord/channel/{id}` and `/api/v1/discord/channel/lookup/{id}` return `200 []` for any ID, including community channel IDs. They look like dead routes from upstream Kemono. `/api/v1/discord/server/{id}` returns 404.

### Website pages with no JSON version

All of these are HTML only. The `/api/v1/...` equivalents return 404.

| Page | Notes |
|---|---|
| `/posts/popular?date=YYYY-MM-DD&period=day\|week\|month&o=N` | Popular posts for a day, week or month |
| `/posts/random`, `/artists/random` | Redirect to a random post or creator |
| `/posts/tags` | Site-wide tag list, linking to `/posts?tag=…` |
| `/artists`, `/artists/updated` | Creator browser (the API equivalent is `/api/v1/creators`) |
| `/missing?o=N` | Creators requested but not archived |
| `/{service}/user/{id}/recommended` | "Similar Creators" tab |
| `/search_hash` | Form front end for `/api/v1/search_hash/{sha256}` |

### Website-only routes (not under `/api/v1`)

These are called from `global.js` and are probably moderator/owner actions. They return HTML or HTMX fragments, not JSON:

- `DELETE /{service}/user/{creator_id}/links`
- `POST /creator_link_requests/{id}/approve`
- `POST /creator_link_requests/{id}/reject`

## Where the spec and reality disagree

1. `/api/schema` is login-gated. The spec is publicly readable at `/api/swagger_schema`.
2. `/app_version` returns `custom` as `text/html`, not a hex commit hash as `text/plain`.
3. **Undocumented endpoints:**
   - `/creators/bans`, `/lookup/cache/{id}`, `/has_pending_dms`, `/importer/logs/{id}`
   - `/{service}/user/{id}/posts` (alias of the bare path)
   - the `hide`/`only` filter params
4. **`/posts` uses a slimmer listing shape:** `substring` instead of `content`, no `added`/`edited`/`embed`.
5. **Undocumented fields everywhere:**
   - posts: `server`, `origin`, `preview_state`, `has_full`, `detail_fetched`, `poll`, `captions`, `tags`
   - creators: `public_id`, `ever_imported`, `relation_id`
6. **Empty results come back as 404, not `[]`:** `/comments` gives `404 {"error":"Not found"}` and `/revisions` gives `404 []`.
7. **`/search_hash` differs from the spec:** it returns `node` + a classic path instead of file metadata, and `discord_posts: null`.
8. **Validation messages don't match the rules:** the `q` minimum length isn't enforced, and the offset error message says "150" but the step is 50.
9. **Timestamps are inconsistent:** `indexed`/`updated` are Unix numbers in `/creators` but ISO strings without a timezone in `/profile` and posts.
10. The spec's `contact` points to `pawchive.st`, but the site runs on `pawchive.pw`.
11. **`tags` is a Postgres array literal string** like `"{a,\"b c\"}"`, not a JSON array.
12. **The `tag`/`hide`/`only` filters don't work on `/api/v1/posts` or `/creators`.** `tag` only works on a creator's post list.
13. **DMs, community chats, popular, random and recommended have no JSON API.** They are HTML only.

## Tooling notes

- Python's `urllib` (default User-Agent) gets **403 from Cloudflare**; `curl` and `reqwest` are fine.

## Not tested

- `POST .../flag`: it puts a post into the site's re-import queue, which affects more than the test account.
- `POST /importer/submit` and `/importer/logs/{id}`: would need a real subscriber session key and import.
- `/account/review_dms` (approving DMs): needs imported DMs on the account.
- Account changes (password, keys) and registration (Turnstile captcha).
- `/creators/bans` and `/lookup/cache`: only returned 404s, so it's unclear whether they're empty or disabled.
- Downloading files with a script. They download fine in a real browser (see "File URLs"); I didn't build a non-browser workaround for the hotlink check.

Favorites were tested on a disposable account and left empty afterwards.
