//! Normalized "artist + title" key, used to recognize the same song coming from
//! different services (YouTube Music, Spotify, local files) and avoid duplicates.
//!
//! Pure functions, no database access.

/// Key for a track, or `None` when the title normalizes to nothing.
/// Only the main artist is kept ("A, B" and "A & B" give the same key as "A").
pub fn match_key(artist: Option<&str>, title: &str) -> Option<String> {
    let title = normalize(&clean_title(title));
    if title.is_empty() {
        return None;
    }
    let artist = artist.map(main_artist).map(normalize).unwrap_or_default();
    Some(format!("{artist}|{title}"))
}

/// Two durations (seconds, 0 when unknown) are close enough to be the same recording.
pub fn same_duration(a: f64, b: f64) -> bool {
    a <= 0.0 || b <= 0.0 || (a - b).abs() <= DURATION_TOLERANCE_SECS
}

const DURATION_TOLERANCE_SECS: f64 = 5.0;

/// Words that mark a bracketed or dashed part of a title as noise.
const NOISE: &[&str] = &[
    "feat",
    "ft",
    "featuring",
    "remaster",
    "remastered",
    "official",
    "audio",
    "video",
    "lyrics",
    "lyric",
    "clip",
    "visualizer",
    "explicit",
    "radio edit",
    "single version",
    "album version",
    "hd",
    "hq",
];

/// Drops `(feat. X)`, `[Official Video]`, `- Remastered 2011`, trailing `feat. X`.
fn clean_title(title: &str) -> String {
    let mut out = String::new();
    let mut rest = title;
    // Bracketed parts that contain a noise word.
    while let Some(start) = rest.find(['(', '[']) {
        let close = if rest[start..].starts_with('(') {
            ')'
        } else {
            ']'
        };
        let Some(len) = rest[start..].find(close) else {
            break;
        };
        let inner = &rest[start + 1..start + len];
        out.push_str(&rest[..start]);
        if !is_noise(inner) {
            out.push_str(&rest[start..=start + len]);
        }
        rest = &rest[start + len + 1..];
    }
    out.push_str(rest);
    // " - Remastered 2011" style suffixes.
    if let Some(dash) = out.rfind(" - ")
        && is_noise(&out[dash + 3..])
    {
        out.truncate(dash);
    }
    // Unbracketed "Title feat. X".
    // ASCII lowercase keeps byte offsets valid for `out`.
    let lower = out.to_ascii_lowercase();
    if let Some(i) = [" feat. ", " feat ", " ft. ", " ft ", " featuring "]
        .iter()
        .filter_map(|marker| lower.find(marker))
        .min()
    {
        out.truncate(i);
    }
    out
}

fn is_noise(part: &str) -> bool {
    let words = normalize(part);
    let words: Vec<&str> = words.split(' ').collect();
    NOISE.iter().any(|noise| {
        let noise: Vec<&str> = noise.split(' ').collect();
        words.windows(noise.len()).any(|w| w == noise.as_slice())
    })
}

/// First artist of "A, B", "A & B", "A feat. B", without YouTube's " - Topic".
fn main_artist(artist: &str) -> &str {
    let artist = artist.strip_suffix(" - Topic").unwrap_or(artist);
    let lower = artist.to_ascii_lowercase();
    let end = [
        ",",
        " & ",
        " feat. ",
        " feat ",
        " ft. ",
        " ft ",
        " featuring ",
    ]
    .iter()
    .filter_map(|sep| lower.find(sep))
    .min()
    .unwrap_or(artist.len());
    &artist[..end]
}

/// Lowercase, accents removed, punctuation turned into single spaces.
fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for c in text.chars().flat_map(char::to_lowercase) {
        let c = fold_accent(c);
        if c.is_alphanumeric() {
            if space && !out.is_empty() {
                out.push(' ');
            }
            space = false;
            out.push(c);
        } else if c != '\'' && c != '’' {
            // "Don't" and "Dont" give the same key.
            space = true;
        }
    }
    out
}

fn fold_accent(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' => 'a',
        'ç' | 'ć' | 'č' => 'c',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ę' | 'ě' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'ī' => 'i',
        'ñ' | 'ń' | 'ň' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' => 'o',
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' => 'u',
        'ý' | 'ÿ' => 'y',
        'š' | 'ś' => 's',
        'ž' | 'ź' | 'ż' => 'z',
        'ł' => 'l',
        'ř' => 'r',
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(artist: &str, title: &str) -> String {
        match_key(Some(artist), title).unwrap()
    }

    #[test]
    fn same_song_from_different_services() {
        let expected = key("Daft Punk", "Get Lucky");
        assert_eq!(
            key(
                "Daft Punk, Pharrell Williams",
                "Get Lucky (feat. Pharrell Williams)"
            ),
            expected
        );
        assert_eq!(key("Daft Punk - Topic", "Get Lucky"), expected);
        assert_eq!(
            key("Daft Punk & Nile Rodgers", "Get Lucky [Official Audio]"),
            expected
        );
        assert_eq!(key("DAFT PUNK", "Get Lucky - Radio Edit"), expected);
        assert_eq!(key("Daft Punk", "Get Lucky feat. Pharrell"), expected);
        assert_eq!(
            key("Queen", "Bohemian Rhapsody - Remastered 2011"),
            key("Queen", "Bohemian Rhapsody")
        );
        assert_eq!(
            key("Stromaé", "Alors on danse"),
            key("Stromae", "Alors On Danse!")
        );
        assert_eq!(
            key("Oasis", "Don't Look Back in Anger"),
            key("Oasis", "Dont Look Back In Anger")
        );
    }

    #[test]
    fn keeps_meaningful_brackets_and_versions() {
        assert_ne!(
            key("The Rolling Stones", "(I Can't Get No) Satisfaction"),
            key("The Rolling Stones", "Satisfaction")
        );
        assert_ne!(key("Muse", "Uprising (Live)"), key("Muse", "Uprising"));
        assert_ne!(key("A", "Song"), key("B", "Song"));
    }

    #[test]
    fn missing_artist_or_title() {
        assert_eq!(match_key(None, "Song").as_deref(), Some("|song"));
        assert_eq!(match_key(Some("A"), " (Official Video) "), None);
    }

    #[test]
    fn durations() {
        assert!(same_duration(200.0, 203.5));
        assert!(!same_duration(200.0, 260.0));
        assert!(same_duration(0.0, 260.0));
    }
}
