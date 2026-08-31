//! The emoji a tag or a group draws inside its node. A word is looked up in the
//! `emojis` crate first — gemoji shortcodes and Unicode names, kept current by
//! someone else — and falls back to the keyword table vendored from Omarchy, which
//! is where the looser synonyms (`music`, `travel`, `idea`) still live. The only
//! thing decided here is which word to look up.

/// The fallback: `<emoji>\t<space-separated keywords>` per line, `#` comments skipped.
const TABLE: &str = include_str!("emojis.txt");

/// Vocabulary neither source can know — jargon, note types, PARA folders — pointed
/// at a plain noun they do describe. Only the hint is ours; the emoji it lands on
/// comes from the crate or the table.
const HINTS: [(&str, &str); 40] = [
    ("anime", "television"),
    ("archive", "package"),
    ("architecture", "building"),
    ("area", "compass"),
    ("banking", "bank"),
    ("cheatsheet", "memo"),
    ("cli", "keyboard"),
    ("concept", "bulb"),
    ("config", "gear"),
    ("containers", "package"),
    ("course", "school"),
    ("decision", "balance"),
    ("dotfiles", "gear"),
    ("editor", "keyboard"),
    ("external", "link"),
    ("finance", "money"),
    ("inbox", "envelope"),
    ("kcna", "anchor"),
    ("kubernetes", "anchor"),
    ("linux", "penguin"),
    ("list", "clipboard"),
    ("meta", "recycle"),
    ("method", "compass"),
    ("nix", "snowflake"),
    ("note", "memo"),
    ("omarchy", "penguin"),
    ("pkm", "brain"),
    ("project", "hammer"),
    ("reference", "books"),
    ("research", "microscope"),
    ("resource", "books"),
    ("router", "house"),
    ("rust", "crab"),
    ("shell", "keyboard"),
    ("skill", "star"),
    ("template", "page"),
    ("terminal", "keyboard"),
    ("untyped", "question"),
    ("vim", "keyboard"),
    ("zettelkasten", "card"),
];

/// The emoji for one tag or group key, or `None` when nothing sensible matches —
/// a node with no icon simply draws as a plain disc.
pub fn of(term: &str) -> Option<&'static str> {
    term.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .find_map(|word| {
            let word = word.to_ascii_lowercase();
            // The table names things in the singular: `idea`, not `ideas`.
            resolve(&word).or_else(|| resolve(word.strip_suffix('s')?))
        })
}

fn resolve(word: &str) -> Option<&'static str> {
    let hinted = HINTS
        .iter()
        .find(|(from, _)| *from == word)
        .map_or(word, |(_, to)| to);
    lookup(hinted)
}

fn lookup(word: &str) -> Option<&'static str> {
    named(word).or_else(|| tabled(word))
}

/// A gemoji shortcode is the name someone already agreed on, so it wins outright;
/// otherwise the emoji whose Unicode name mentions `word` earliest.
fn named(word: &str) -> Option<&'static str> {
    if let Some(emoji) = emojis::get_by_shortcode(word) {
        return Some(emoji.as_str());
    }
    emojis::iter()
        .filter_map(|emoji| {
            let pos = emoji.name().split(' ').position(|w| w == word)?;
            Some((pos, emoji.as_str()))
        })
        .min_by_key(|(pos, _)| *pos)
        .map(|(_, emoji)| emoji)
}

/// The best emoji whose keywords contain `word`. A keyword earlier in an emoji's
/// name describes it better — that is what keeps `keyboard` off the piano.
fn tabled(word: &str) -> Option<&'static str> {
    let mut best: Option<((usize, usize), &'static str)> = None;
    for (line_no, line) in TABLE.lines().filter(|l| !l.starts_with('#')).enumerate() {
        let Some((emoji, keywords)) = line.split_once('\t') else {
            continue;
        };
        if let Some(pos) = keywords.split(' ').position(|k| k == word) {
            let rank = (pos, line_no);
            if best.is_none_or(|(seen, _)| rank < seen) {
                best = Some((rank, emoji));
            }
        }
    }
    best.map(|(_, emoji)| emoji)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_keyword_earlier_in_the_name_wins() {
        assert_eq!(lookup("keyboard"), Some("⌨️"), "not the musical keyboard");
        assert_eq!(lookup("star"), Some("⭐"));
        assert_eq!(lookup("penguin"), Some("🐧"));
        assert_eq!(lookup("nonsenseword"), None);
    }

    #[test]
    fn the_crate_answers_first_and_the_table_covers_what_it_cannot() {
        assert_eq!(named("book"), Some("\u{1f4d6}"), "the gemoji shortcode");
        assert_eq!(tabled("book"), Some("\u{1f4d5}"), "the table would disagree");
        assert_eq!(lookup("book"), named("book"), "so the crate wins");

        assert_eq!(named("travel"), None, "no shortcode, no Unicode name");
        assert_eq!(lookup("travel"), tabled("travel"), "the table still knows it");
    }

    #[test]
    fn hints_carry_jargon_to_a_noun_the_table_knows() {
        assert_eq!(of("kubernetes"), of("anchor"));
        assert_eq!(of("vim"), Some("⌨️"));
        assert_eq!(of("pkm"), Some("🧠"));
    }

    #[test]
    fn compound_terms_fall_through_to_the_first_word_that_resolves() {
        assert_eq!(of("cloud-native"), of("cloud"));
        assert_eq!(
            of("course-note"),
            of("school"),
            "the hint beats the bare word"
        );
        assert_eq!(of("Portugal"), of("portugal"), "case does not matter");
        assert_eq!(
            of("ideas"),
            of("idea"),
            "a plural falls back to its singular"
        );
        assert_eq!(of("qqqq-wwww"), None, "zzz would have matched \u{1f4a4}");
    }
}
