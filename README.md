# kemono-project

Rust crate `kemono-core`: types and HTTP clients for Kemono and its forks.

| Module | What | Source of truth |
|---|---|---|
| `kemono` | `KemonoClient` for kemono.su / coomer.su | hand-maintained against `schema/kemono.yaml` |
| `pawchive` | `PawchiveClient` for <https://pawchive.pw> | hand-written; `schema/pawchive.yaml`, `docs/pawchive-api.md` |
| `client` | `HttpCore`: client + base URL + status mapping, shared by fork clients | |
| `handles` | `Creator` / `PostRef` / `PostsQuery` objects over both clients | |
| `error` | `Error { Http, NotFound, BadRequest, Unauthorized, RateLimited, Status }` | |

## Quick start

```rust
use kemono_core::pawchive::PawchiveClient;

#[tokio::main]
async fn main() -> Result<(), kemono_core::error::Error> {
    let c = PawchiveClient::default(); // https://pawchive.pw/api

    let profile = c.get_profile("patreon", "110390090").await?;
    let page = c.get_user_posts("patreon", "110390090", None, Some(0), None).await?; // 50 per page
    let all = c.get_all_user_posts("patreon", "110390090", Some("Steins;Gate"), None, Some(10)).await?;
    let post = c.get_post("patreon", "110390090", "176121337").await?;
    println!("{} {:?} {:?}", profile.name, post.parsed_tags(), post.file.url());
    Ok(())
}
```

The same through handles, which bundle `service` + `id` with the client:

```rust
let creator = c.creator("patreon", "110390090");      // no request yet
let tags = creator.tags().await?;
let mut posts = creator.posts().tag("Steins;Gate").max_pages(10);
while let Some(p) = posts.next().await? { println!("{}", p.title); } // lazy, 50 per request
let all = creator.posts().collect().await?;            // or everything at once
let comments = creator.post("176121337").comments().await?;
let handle = profile.bind(&c);                         // from any fetched profile or post
```

`KemonoClient::creator(..)` offers the same for posts (Kemono has no profile/tags endpoint in the schema).
The `get_*` client methods remain as the low-level layer.

Run the bundled example: `cargo run -p kemono-core --example pawchive`.

## Things worth knowing

- **Pagination:** the offset `o` must be a multiple of 50 (otherwise `Error::BadRequest`).
  `get_all_user_posts` walks the pages; pass `max_pages` for big creators.
- **Empty results:** comments and revisions return a 404 when there are none; the client turns that into `Vec::new()`.
- **Tags** come back as a Postgres array literal string; use `PawchivePost::parsed_tags()`.
- **Timestamps** are `jiff` types. Most are ISO strings without a timezone (`jiff::civil::DateTime`);
  `get_creators` uses Unix seconds (`jiff::Timestamp`).
- **File URLs:** `PawchiveFile::url()` builds `{server}/data{path}?f={name}`. The file hosts are hotlink-protected:
  a plain request gets 403 without a `Referer` from pawchive.pw. The crate does not work around this; respect the site's terms.
  Listing items with `preview_only: true` have no URL.
- **Timeouts and retries:** requests time out after 30 s (120 s for `get_creators`). GETs and the idempotent favorite calls are retried up to 3 times with backoff on connection errors, timeouts, 429 and 5xx, honouring `Retry-After` (max 30 s); a final 429 is `Error::RateLimited`. Change it with `PawchiveClient::with_retry(RetryPolicy { .. })` or turn it off with `RetryPolicy::NONE`. `login` is never retried.
- **`get_creators()`** downloads the whole site list (about 16 MB, about 100k entries).

## Accounts

There is no JSON login. `login(user, pass)` posts the website form and keeps the `session` cookie in the client:

```rust
c.login(&user, &pass).await?;           // Err(Error::Unauthorized) if rejected
let favs = c.favorite_creators().await?;
c.add_favorite_post("patreon", "110390090", "176121337").await?; // 204, idempotent
```

`add_favorite_*` first checks that the creator or post exists (the server accepts any ID) and returns `Error::NotFound` without mutating if not. Removing is a harmless no-op, so it is not checked.

Not implemented on purpose (they change data on the site): flagging a post for re-import and the importer.

## Development

```sh
cargo fmt && cargo clippy --all-targets && cargo test
scripts/validate-schema.sh                 # validates schema/*.yaml as OpenAPI (needs python3)

# live tests hit the real site and are #[ignore]d
cargo test -- --ignored
# account tests also need a disposable account:
PAWCHIVE_USER=... PAWCHIVE_PASS=... cargo test -- --ignored live_account
```
