use bindings::{Windows::Foundation::Uri, Windows::Web::Syndication::SyndicationClient};

pub fn print_feeds() -> windows::Result<()> {
    let uri = Uri::CreateUri("https://blogs.windows.com/feed")?;
    let client = SyndicationClient::new()?;
    let feed = client.RetrieveFeedAsync(uri)?.get()?;

    for item in feed.Items()? {
        println!("{}", item.Title()?.Text()?);
    }

    Ok(())
}
