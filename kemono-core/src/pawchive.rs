//! Client for Pawchive (<https://pawchive.pw>), a Kemono fork.
//!
//! ```no_run
//! # #[cfg(feature = "reqwest")]
//! # async fn demo() -> Result<(), kemono_core::error::Error> {
//! use kemono_core::pawchive::PawchiveClient;
//! let c = PawchiveClient::default();
//! let post = c.get_post("patreon", "110390090", "176121337").await?;
//! println!("{} {:?}", post.title, post.parsed_tags());
//! # Ok(()) }
//! ```
//!
//! Endpoints that answer 404 for "none" (comments, revisions) return an empty `Vec`.
//! Account calls need [`PawchiveClient::login`] first and otherwise fail with
//! [`Error::Unauthorized`].

use std::time::Duration;

use std::sync::Arc;

use crate::client::{HttpCore, RetryPolicy};
use crate::error::Error;
use crate::transport::{Method, Transport};
use crate::types::pawchive::{
    PawchiveAnnouncement, PawchiveComment, PawchiveCreatorListItem, PawchiveCreatorProfile,
    PawchiveFancard, PawchiveFavoriteCreator, PawchiveFavoritePost, PawchiveHashSearch,
    PawchivePost, PawchivePostSummary, PawchiveRevision, PawchiveTagCount,
};

/// Pawchive serves 50 results per page; `o` must be a multiple of this.
pub use crate::handles::paging::PAGE_SIZE;

#[derive(Debug, Clone)]
pub struct PawchiveClient {
    http: HttpCore,
}

#[cfg(feature = "reqwest")]
impl Default for PawchiveClient {
    fn default() -> Self {
        Self::new("https://pawchive.pw/api")
    }
}

impl PawchiveClient {
    /// `base_url` is e.g. `https://pawchive.pw/api` (no trailing slash).
    ///
    /// Uses [`ReqwestTransport`](crate::transport::ReqwestTransport) (feature `reqwest`).
    /// For wasm or another HTTP stack use [`with_transport`](Self::with_transport).
    #[cfg(feature = "reqwest")]
    pub fn new(base_url: &str) -> Self {
        Self::with_transport(
            base_url,
            Arc::new(crate::transport::ReqwestTransport::new()),
        )
    }

    /// A client that sends its requests through `transport`.
    pub fn with_transport(base_url: &str, transport: Arc<dyn Transport>) -> Self {
        Self {
            http: HttpCore::new(base_url, transport),
        }
    }

    /// Replaces the retry policy (default: 3 attempts, 500 ms doubling backoff; see [`RetryPolicy`]).
    /// Applies to GETs and the idempotent favorite calls; never to [`login`](Self::login).
    pub fn with_retry(mut self, policy: RetryPolicy) -> Self {
        self.http = self.http.with_retry(policy);
        self
    }

    /// Restores a session returned by an earlier [`login`](Self::login).
    pub fn with_session(self, session: impl Into<String>) -> Self {
        self.http.set_session(Some(session.into()));
        self
    }

    /// The current `session` cookie value, if logged in. Store it to skip logging in next time.
    pub fn session(&self) -> Option<String> {
        self.http.session()
    }

    /// Log in through the website form (there is no JSON login). On success the
    /// `session` cookie is kept for later account calls and also returned, so it can
    /// be persisted and restored with [`with_session`](Self::with_session). A rejected
    /// login is [`Error::Unauthorized`].
    pub async fn login(&self, username: &str, password: &str) -> Result<String, Error> {
        let root = self.http.base_url().trim_end_matches("/api");
        let resp = self
            .http
            .post_form(
                format!("{root}/account/login"),
                &[
                    ("username", username),
                    ("password", password),
                    ("location", "/artists"),
                ],
            )
            .await?;
        let location = resp.header("location").unwrap_or("");
        let redirect = (300..400).contains(&resp.status);
        // Success redirects to /artists?logged_in=yes; failure back to /account/login.
        if redirect && !location.starts_with("/account/login") {
            let session = resp
                .headers_all("set-cookie")
                .filter_map(|c| c.split(';').next()?.trim().strip_prefix("session="))
                .find(|v| !v.is_empty())
                .map(str::to_owned);
            // A success without a session cookie is of no use to us.
            let session = session.ok_or(Error::Unauthorized)?;
            self.http.set_session(Some(session.clone()));
            Ok(session)
        } else if redirect {
            Err(Error::Unauthorized)
        } else {
            Error::check(resp).and(Err(Error::Unauthorized))
        }
    }

    /// `GET /v1/account/favorites?type=artist` (needs login; else [`Error::Unauthorized`]).
    pub async fn favorite_creators(&self) -> Result<Vec<PawchiveFavoriteCreator>, Error> {
        let url = format!("{}/v1/account/favorites", self.http.base_url());
        self.http.get_json(url, &[("type", "artist".into())]).await
    }

    /// `GET /v1/account/favorites?type=post` (needs login).
    pub async fn favorite_posts(&self) -> Result<Vec<PawchiveFavoritePost>, Error> {
        let url = format!("{}/v1/account/favorites", self.http.base_url());
        self.http.get_json(url, &[("type", "post".into())]).await
    }

    /// `POST /v1/favorites/creator/{service}/{id}`. Idempotent (and retried).
    ///
    /// The server accepts any ID, so this first checks that the creator exists
    /// ([`get_profile`](Self::get_profile)); an unknown one is [`Error::NotFound`] and nothing is changed.
    pub async fn add_favorite_creator(&self, service: &str, creator_id: &str) -> Result<(), Error> {
        self.get_profile(service, creator_id).await?;
        self.fav_creator(Method::Post, service, creator_id).await
    }

    /// `DELETE /v1/favorites/creator/{service}/{id}`. Idempotent (removing a non-favorite is a no-op), so no existence check.
    pub async fn remove_favorite_creator(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<(), Error> {
        self.fav_creator(Method::Delete, service, creator_id).await
    }

    /// `POST /v1/favorites/post/{service}/{creator_id}/{post_id}`. Idempotent (and retried).
    ///
    /// Checks that the post exists first ([`get_post`](Self::get_post)); an unknown one is
    /// [`Error::NotFound`] and nothing is changed.
    pub async fn add_favorite_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<(), Error> {
        self.get_post(service, creator_id, post_id).await?;
        self.fav_post(Method::Post, service, creator_id, post_id)
            .await
    }

    /// `DELETE /v1/favorites/post/{service}/{creator_id}/{post_id}`. Idempotent.
    pub async fn remove_favorite_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<(), Error> {
        self.fav_post(Method::Delete, service, creator_id, post_id)
            .await
    }

    async fn fav_creator(&self, m: Method, service: &str, id: &str) -> Result<(), Error> {
        let url = format!(
            "{}/v1/favorites/creator/{service}/{id}",
            self.http.base_url()
        );
        self.http.send_no_content(m, url).await
    }

    async fn fav_post(&self, m: Method, service: &str, id: &str, post: &str) -> Result<(), Error> {
        let url = format!(
            "{}/v1/favorites/post/{service}/{id}/{post}",
            self.http.base_url()
        );
        self.http.send_no_content(m, url).await
    }

    /// `GET /v1/has_pending_dms`; `false` when logged out.
    pub async fn has_pending_dms(&self) -> Result<bool, Error> {
        let url = format!("{}/v1/has_pending_dms", self.http.base_url());
        self.http.get_json(url, &[]).await
    }

    /// A creator's posts (full shape), 50 per page.
    /// `GET /v1/{service}/user/{creator_id}/posts`
    ///
    /// `offset` must be a multiple of [`PAGE_SIZE`]. `tag` is an exact tag name.
    /// `q` is a full-text search; no match yields an empty list.
    pub async fn get_user_posts(
        &self,
        service: &str,
        creator_id: &str,
        tag: Option<&str>,
        offset: Option<u32>,
        q: Option<&str>,
    ) -> Result<Vec<PawchivePost>, Error> {
        let url = format!(
            "{}/v1/{}/user/{}/posts",
            self.http.base_url(),
            service,
            creator_id
        );
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(t) = tag {
            params.push(("tag", t.to_string()));
        }
        if let Some(o) = offset {
            params.push(("o", o.to_string()));
        }
        if let Some(q) = q {
            params.push(("q", q.to_string()));
        }
        self.http.get_json(url, &params).await
    }

    /// Fetches every page of a creator's posts, 50 at a time, until a short page.
    ///
    /// `max_pages` caps the number of requests (`None` = unbounded; a large creator
    /// can mean hundreds of requests). Filters behave as in [`get_user_posts`](Self::get_user_posts).
    pub async fn get_all_user_posts(
        &self,
        service: &str,
        creator_id: &str,
        tag: Option<&str>,
        q: Option<&str>,
        max_pages: Option<u32>,
    ) -> Result<Vec<PawchivePost>, Error> {
        let mut query = self.creator(service, creator_id).posts();
        if let Some(t) = tag {
            query = query.tag(t);
        }
        if let Some(q) = q {
            query = query.query(q);
        }
        if let Some(m) = max_pages {
            query = query.max_pages(m);
        }
        query.collect().await
    }

    /// `GET /v1/{service}/user/{creator_id}/post/{post_id}`
    pub async fn get_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<PawchivePost, Error> {
        let url = format!(
            "{}/v1/{}/user/{}/post/{}",
            self.http.base_url(),
            service,
            creator_id,
            post_id
        );
        self.http.get_json(url, &[]).await
    }

    /// Recent posts site-wide (listing shape), 50 per page. `GET /v1/posts`
    pub async fn get_posts(
        &self,
        q: Option<&str>,
        offset: Option<u32>,
    ) -> Result<Vec<PawchivePostSummary>, Error> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(q) = q {
            params.push(("q", q.to_string()));
        }
        if let Some(o) = offset {
            params.push(("o", o.to_string()));
        }
        self.http
            .get_json(format!("{}/v1/posts", self.http.base_url()), &params)
            .await
    }

    /// Every creator on the site in one response (~16 MB). `GET /v1/creators`
    pub async fn get_creators(&self) -> Result<Vec<PawchiveCreatorListItem>, Error> {
        // ~16 MB: allow a slower download than the default timeout.
        let url = format!("{}/v1/creators", self.http.base_url());
        self.http
            .get_json_timeout(url, &[], Some(Duration::from_secs(120)))
            .await
    }

    fn user_url(&self, service: &str, creator_id: &str, tail: &str) -> String {
        format!(
            "{}/v1/{}/user/{}/{}",
            self.http.base_url(),
            service,
            creator_id,
            tail
        )
    }

    /// `GET /v1/{service}/user/{creator_id}/tags`
    pub async fn get_tags(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<PawchiveTagCount>, Error> {
        self.http
            .get_json(self.user_url(service, creator_id, "tags"), &[])
            .await
    }

    /// Linked accounts of the creator. `GET /v1/{service}/user/{creator_id}/links`
    pub async fn get_links(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<PawchiveCreatorProfile>, Error> {
        self.http
            .get_json(self.user_url(service, creator_id, "links"), &[])
            .await
    }

    /// The creator's announcements (HTML content; `[]` when none).
    /// `GET /v1/{service}/user/{creator_id}/announcements`
    pub async fn get_announcements(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<PawchiveAnnouncement>, Error> {
        self.http
            .get_json(self.user_url(service, creator_id, "announcements"), &[])
            .await
    }

    /// Fanbox fancards (other services return `[]`). `GET /v1/{service}/user/{creator_id}/fancards`
    pub async fn get_fancards(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<PawchiveFancard>, Error> {
        self.http
            .get_json(self.user_url(service, creator_id, "fancards"), &[])
            .await
    }

    /// Comments on a post; the server's 404 for "none" becomes an empty list.
    pub async fn get_comments(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<PawchiveComment>, Error> {
        let url = self.user_url(service, creator_id, &format!("post/{post_id}/comments"));
        empty_on_404(self.http.get_json(url, &[]).await)
    }

    /// Revisions, newest first; the server's 404 for "none" becomes an empty list.
    pub async fn get_revisions(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<PawchiveRevision>, Error> {
        let url = self.user_url(service, creator_id, &format!("post/{post_id}/revisions"));
        empty_on_404(self.http.get_json(url, &[]).await)
    }

    /// Posts containing a file with this SHA-256. `GET /v1/search_hash/{sha256}`
    pub async fn search_hash(&self, sha256: &str) -> Result<PawchiveHashSearch, Error> {
        self.http
            .get_json(
                format!("{}/v1/search_hash/{}", self.http.base_url(), sha256),
                &[],
            )
            .await
    }

    /// `GET /v1/{service}/user/{creator_id}/profile`
    pub async fn get_profile(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<PawchiveCreatorProfile, Error> {
        let url = format!(
            "{}/v1/{}/user/{}/profile",
            self.http.base_url(),
            service,
            creator_id
        );
        self.http.get_json(url, &[]).await
    }
}

fn empty_on_404<T>(r: Result<Vec<T>, Error>) -> Result<Vec<T>, Error> {
    match r {
        Err(Error::NotFound) => Ok(Vec::new()),
        other => other,
    }
}

#[cfg(test)]
mod mocked {
    use super::*;
    use crate::transport::mock::{MockTransport, resp, resp_h};

    fn client(t: &Arc<MockTransport>) -> PawchiveClient {
        PawchiveClient::with_transport("http://x/api", t.clone())
    }

    #[tokio::test]
    async fn login_captures_session_and_sends_it_afterwards() {
        let t = MockTransport::new(|r, _| {
            Ok(if r.url.ends_with("/account/login") {
                resp_h(
                    303,
                    &[
                        ("Location", "/artists?logged_in=yes"),
                        ("Set-Cookie", "other=1; Path=/"),
                        ("Set-Cookie", "session=tok123; HttpOnly; Path=/"),
                    ],
                    "",
                )
            } else {
                resp(200, "true")
            })
        });
        let c = client(&t);
        assert_eq!(c.login("a b", "p&w").await.unwrap(), "tok123");
        assert_eq!(c.session().as_deref(), Some("tok123"));
        assert!(c.has_pending_dms().await.unwrap());

        let reqs = t.requests();
        assert_eq!(reqs[0].url, "http://x/account/login");
        assert_eq!(
            String::from_utf8(reqs[0].body.clone().unwrap()).unwrap(),
            "username=a+b&password=p%26w&location=%2Fartists"
        );
        assert!(
            reqs[1]
                .headers
                .iter()
                .any(|(k, v)| k == "Cookie" && v == "session=tok123")
        );
    }

    #[tokio::test]
    async fn rejected_login_is_unauthorized_and_stores_nothing() {
        let t =
            MockTransport::scripted(vec![Ok(resp_h(303, &[("Location", "/account/login")], ""))]);
        let c = client(&t);
        assert!(matches!(c.login("a", "b").await, Err(Error::Unauthorized)));
        assert_eq!(c.session(), None);

        let t = MockTransport::scripted(vec![Ok(resp(200, "login page"))]);
        assert!(matches!(
            client(&t).login("a", "b").await,
            Err(Error::Unauthorized)
        ));
    }

    #[tokio::test]
    async fn with_session_is_shared_by_clones() {
        let t = MockTransport::scripted(vec![Ok(resp(200, "false"))]);
        let c = client(&t).with_session("s1");
        let _ = c.clone().has_pending_dms().await.unwrap();
        assert!(
            t.requests()[0]
                .headers
                .iter()
                .any(|(k, v)| k == "Cookie" && v == "session=s1")
        );
    }

    /// Serves `pages` posts-pages of the given sizes, keyed by the `o` query parameter.
    fn posts_transport(sizes: &'static [usize]) -> Arc<MockTransport> {
        MockTransport::new(move |r, _| {
            let o: usize = r
                .url
                .split("o=")
                .nth(1)
                .map(|s| s.split('&').next().unwrap().parse().unwrap())
                .unwrap_or(0);
            let n = sizes.get(o / 50).copied().unwrap_or(0);
            let posts: Vec<_> = (0..n)
                .map(|i| serde_json::json!({"id": format!("{}", o + i), "user": "u", "service": "s"}))
                .collect();
            Ok(resp(200, &serde_json::to_string(&posts).unwrap()))
        })
    }

    #[tokio::test]
    async fn handle_posts_walk_pages_in_order_and_stop_on_short_page() {
        let t = posts_transport(&[50, 50, 3]);
        let creator = client(&t).creator("s", "u");
        let all = creator
            .posts()
            .tag("a b")
            .query("q")
            .collect()
            .await
            .unwrap();
        assert_eq!(all.len(), 103);
        assert_eq!(all[50].id, "50");
        let reqs = t.requests();
        assert_eq!(reqs.len(), 3);
        assert!(
            reqs[2]
                .url
                .starts_with("http://x/api/v1/s/user/u/posts?tag=a+b&o=100&q=q"),
            "{}",
            reqs[2].url
        );
    }

    #[tokio::test]
    async fn handle_posts_max_pages_and_single_page() {
        let t = posts_transport(&[50, 50, 50]);
        let creator = client(&t).creator("s", "u");
        assert_eq!(
            creator.posts().max_pages(2).collect().await.unwrap().len(),
            100
        );
        assert_eq!(t.requests().len(), 2);
        assert_eq!(creator.posts().page(1).await.unwrap()[0].id, "50");
        let mut q = creator.posts().max_pages(1);
        assert_eq!(q.next().await.unwrap().unwrap().id, "0");
    }

    #[tokio::test]
    async fn comments_404_is_empty_via_handle() {
        let t = MockTransport::scripted(vec![Ok(resp(404, ""))]);
        let post = client(&t).creator("s", "u").post("1");
        assert!(post.comments().await.unwrap().is_empty());
        assert!(post.revisions().await.unwrap().is_empty());
        assert_eq!(
            t.requests()[0].url,
            "http://x/api/v1/s/user/u/post/1/comments"
        );
    }
}

#[cfg(all(test, feature = "reqwest"))]
mod live {
    use super::*;

    #[tokio::test]
    #[ignore = "hits pawchive.pw"]
    async fn live_roundtrip() {
        let c = PawchiveClient::default();
        let prof = c.get_profile("patreon", "110390090").await.unwrap();
        assert_eq!(prof.id, "110390090");
        let posts = c
            .get_user_posts("patreon", "110390090", None, Some(0), None)
            .await
            .unwrap();
        assert!(!posts.is_empty());
        let p = c
            .get_post("patreon", "110390090", "176121337")
            .await
            .unwrap();
        assert_eq!(p.id, "176121337");
    }

    #[tokio::test]
    #[ignore = "hits pawchive.pw"]
    async fn live_handles() {
        let creator = PawchiveClient::default().creator("patreon", "110390090");
        assert_eq!(creator.profile().await.unwrap().id, "110390090");
        assert!(!creator.tags().await.unwrap().is_empty());
        let mut posts = creator.posts().max_pages(1);
        let first = posts.next().await.unwrap().unwrap();
        assert_eq!(first.user, "110390090");
        assert_eq!(creator.posts().page(0).await.unwrap()[0].id, first.id);
        assert_eq!(creator.post(&first.id).fetch().await.unwrap().id, first.id);
    }

    #[tokio::test]
    #[ignore = "hits pawchive.pw"]
    async fn live_error_mapping() {
        let c = PawchiveClient::default();
        let r = c
            .get_user_posts("patreon", "110390090", None, Some(25), None)
            .await;
        assert!(matches!(r, Err(Error::BadRequest(_))), "{r:?}");
        let r = c.get_post("patreon", "110390090", "0").await;
        assert!(matches!(r, Err(Error::NotFound)), "{r:?}");
    }

    #[tokio::test]
    #[ignore = "hits pawchive.pw"]
    async fn live_read_only() {
        let c = PawchiveClient::default();
        assert_eq!(c.get_posts(None, Some(0)).await.unwrap().len(), 50);
        let tags = c.get_tags("patreon", "110390090").await.unwrap();
        assert!(!tags.is_empty());
        let links = c.get_links("fanbox", "37736420").await.unwrap();
        assert!(links.iter().any(|l| l.relation_id.is_some()));
        assert!(
            !c.get_fancards("fanbox", "37736420")
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            c.get_announcements("patreon", "110390090")
                .await
                .unwrap()
                .is_empty()
        );
        let ann = c.get_announcements("patreon", "3295915").await.unwrap();
        assert!(!ann.is_empty() && ann[0].user_id == "3295915");
        // no comments / revisions -> empty, not an error
        assert!(
            c.get_comments("patreon", "110390090", "176121337")
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            c.get_revisions("patreon", "110390090", "176121337")
                .await
                .unwrap()
                .is_empty()
        );
        let found = c
            .get_comments("patreon", "90235213", "176154456")
            .await
            .unwrap();
        assert!(!found.is_empty());
        let h = "44ad9682c02ecfec5b55e0d06cbba9d4d9d11d10895ee859be2588a714d33577";
        let res = c.search_hash(h).await.unwrap();
        assert!(res.posts[0].file.url().is_some());
        let all = c.get_creators().await.unwrap();
        assert!(all.len() > 90_000);
    }

    /// Needs `PAWCHIVE_USER` / `PAWCHIVE_PASS` (a disposable account). Adds then removes one favorite.
    #[tokio::test]
    #[ignore = "hits pawchive.pw with an account"]
    async fn live_account() {
        let (Ok(u), Ok(p)) = (
            std::env::var("PAWCHIVE_USER"),
            std::env::var("PAWCHIVE_PASS"),
        ) else {
            panic!("set PAWCHIVE_USER and PAWCHIVE_PASS");
        };
        let c = PawchiveClient::default();
        assert!(matches!(
            c.favorite_creators().await,
            Err(Error::Unauthorized)
        ));
        assert!(matches!(
            c.login(&u, "definitely-wrong").await,
            Err(Error::Unauthorized)
        ));
        c.login(&u, &p).await.unwrap();
        let _ = c.has_pending_dms().await.unwrap();
        let (svc, id) = ("patreon", "110390090");
        // unknown targets are rejected before anything is mutated
        assert!(matches!(
            c.add_favorite_creator(svc, "0").await,
            Err(Error::NotFound)
        ));
        assert!(matches!(
            c.add_favorite_post(svc, id, "0").await,
            Err(Error::NotFound)
        ));
        assert!(c.favorite_creators().await.unwrap().is_empty());
        c.add_favorite_creator(svc, id).await.unwrap();
        let got = c.favorite_creators().await;
        c.remove_favorite_creator(svc, id).await.unwrap(); // clean up before asserting
        assert!(got.unwrap().iter().any(|f| f.profile.id == id));
        c.add_favorite_post(svc, id, "176121337").await.unwrap();
        let got = c.favorite_posts().await;
        c.remove_favorite_post(svc, id, "176121337").await.unwrap();
        let got = got.unwrap();
        assert!(got.iter().any(|f| f.post.id == "176121337"));
        assert!(got[0].post.file.url().is_some());
    }

    #[tokio::test]
    #[ignore = "hits pawchive.pw"]
    async fn live_pagination() {
        let c = PawchiveClient::default();
        let two = c
            .get_all_user_posts("patreon", "110390090", None, None, Some(2))
            .await
            .unwrap();
        let p0 = c
            .get_user_posts("patreon", "110390090", None, Some(0), None)
            .await
            .unwrap();
        let p1 = c
            .get_user_posts("patreon", "110390090", None, Some(50), None)
            .await
            .unwrap();
        let expect: Vec<_> = p0.iter().chain(&p1).map(|p| p.id.clone()).collect();
        let got: Vec<_> = two.iter().map(|p| p.id.clone()).collect();
        assert_eq!(got, expect);
        // a filter that matches little ends on the first short page
        let tagged = c
            .get_all_user_posts("patreon", "110390090", Some("Steins;Gate"), None, None)
            .await
            .unwrap();
        assert!(tagged.len() < 50 && !tagged.is_empty());
    }
}
