use include_dir::{Dir, include_dir};
use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, html};

static DOCS: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../docs");

pub struct Doc {
    pub file: &'static str,
    /// The channel slug: `03-bots.md` is published to `dev-bots`.
    pub slug: &'static str,
    pub markdown: &'static str,
}

fn slug_of(file: &str) -> Option<&str> {
    let (number, rest) = file.split_once('-')?;
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.strip_suffix(".md")
}

pub fn all() -> Vec<Doc> {
    let mut docs: Vec<Doc> = DOCS
        .files()
        .filter_map(|file| {
            let name = file.path().file_name()?.to_str()?;
            Some(Doc { file: name, slug: slug_of(name)?, markdown: file.contents_utf8()? })
        })
        .collect();
    docs.sort_by_key(|doc| doc.file);
    docs
}

/// Links between docs point at their channels in the niche.
fn rewrite_link(href: &str) -> Option<String> {
    let (file, hash) = href.split_once('#').map_or((href, None), |(f, h)| (f, Some(h)));
    let slug = slug_of(file.trim_start_matches("./"))?;
    Some(match hash {
        Some(hash) => format!("/niche/dev/{slug}#{hash}"),
        None => format!("/niche/dev/{slug}"),
    })
}

/// Renders a doc to HTML. Raw HTML in the markdown is shown as text.
pub fn render(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let events = Parser::new_ext(markdown, options).map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(raw),
        Event::Start(Tag::Link { link_type, dest_url, title, id }) => {
            let dest_url = rewrite_link(&dest_url).map(CowStr::from).unwrap_or(dest_url);
            Event::Start(Tag::Link { link_type, dest_url, title, id })
        }
        other => other,
    });
    let mut out = String::new();
    html::push_html(&mut out, events);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_doc_has_a_channel_slug() {
        let docs = all();
        assert!(docs.len() >= 8);
        assert!(docs.iter().any(|doc| doc.slug == "tawkathon"));
    }

    #[test]
    fn links_between_docs_point_at_channels() {
        let html = render("See [games](06-games.md#lobbies) and [site](https://tawks.app).");
        assert!(html.contains(r#"href="/niche/dev/games#lobbies""#));
        assert!(html.contains(r#"href="https://tawks.app""#));
    }

    #[test]
    fn raw_html_is_escaped() {
        assert!(render("<script>x</script>").contains("&lt;script&gt;"));
    }
}
