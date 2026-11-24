//! `cargo run -p kemono-core --example pawchive`
use kemono_core::pawchive::PawchiveClient;

#[tokio::main]
async fn main() -> Result<(), kemono_core::error::Error> {
    let creator = PawchiveClient::default().creator("patreon", "110390090");

    let profile = creator.profile().await?;
    println!("{} (updated {})", profile.name, profile.updated);
    println!("{} tags", creator.tags().await?.len());

    let mut posts = creator.posts().max_pages(1);
    let mut n = 0;
    let mut first = None;
    while let Some(p) = posts.next().await? {
        n += 1;
        first.get_or_insert(p);
    }
    println!("{n} posts on the first page");

    if let Some(first) = first {
        let post = creator.post(&first.id).fetch().await?;
        println!(
            "{:?} tags={:?} file={:?}",
            post.title,
            post.parsed_tags(),
            post.file.url()
        );
    }
    Ok(())
}
