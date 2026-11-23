//! `cargo run -p kemono-core --example pawchive`
use kemono_core::pawchive::PawchiveClient;

#[tokio::main]
async fn main() -> Result<(), kemono_core::error::Error> {
    let c = PawchiveClient::default();

    let profile = c.get_profile("patreon", "110390090").await?;
    println!("{} (updated {})", profile.name, profile.updated);

    let posts = c
        .get_user_posts("patreon", "110390090", None, Some(0), None)
        .await?;
    println!("{} posts on the first page", posts.len());

    if let Some(first) = posts.first() {
        let post = c.get_post("patreon", &first.user, &first.id).await?;
        println!(
            "{:?} tags={:?} file={:?}",
            post.title,
            post.parsed_tags(),
            post.file.url()
        );
    }
    Ok(())
}
