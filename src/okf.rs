//! What a concept declares about itself, and what that means: the Open Knowledge Format's
//! frontmatter (OKF v0.2 §4.1, §5, §13), typed, and the rules the spec asks a consumer to
//! apply to it — which trust tier `verified` earns, whether `stale_after` has passed, and
//! which filenames are not concepts at all.
//!
//! [`crate::yaml`] turns the text into a value tree; this module is where a key name is
//! spelled and given a meaning. Every rule is a pure function over what a note said, so
//! the spec's sections are checkable one test at a time.

use crate::yaml::Yaml;
use brain_map_model::{Actor, Concept, Provenance};
use std::time::{SystemTime, UNIX_EPOCH};

/// A concept with no `type` is still a concept: §11 forbids rejecting it, so it groups
/// under one name instead of vanishing.
pub const UNTYPED: &str = "Untyped";
/// `index.md` and `log.md` are reserved (§3.1). They are not concepts, so they carry no
/// type and group together rather than diluting [`UNTYPED`].
pub const RESERVED: &str = "Index & log";

/// Every frontmatter key the graph reads, typed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Front {
    pub title: Option<String>,
    pub tags: Vec<String>,
    /// The emoji the note declares. Nothing guesses one: this is the only source.
    pub icon: Option<String>,
    /// The concept's `type`, the one key OKF requires (§4.1).
    pub concept: Option<String>,
    /// `draft`, `stable` or `deprecated` (§5.4). Absent means stable.
    pub status: Option<String>,
    /// The instant the content goes stale (§5.5).
    pub stale_after: Option<String>,
    /// What the reader shows: the prose, the asset, the actors, the sources.
    pub about: Concept,
}

impl Front {
    pub fn read(yaml: &Yaml) -> Front {
        let some = |key: &str| Some(yaml.text(key).to_string()).filter(|v| !v.is_empty());
        let actor = |value: &Yaml| Actor {
            by: value.text("by").to_string(),
            at: value.text("at").to_string(),
        };
        let strings = |key: &str| -> Vec<String> {
            yaml.get(key)
                .map(|value| {
                    value
                        .items()
                        .iter()
                        .filter_map(|v| v.str())
                        .filter(|v| !v.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default()
        };
        // §13.1: a v0.1 `timestamp` stands in for `generated.at` when `generated` is absent.
        let generated = match yaml.get("generated") {
            Some(value) => Some(actor(value)),
            None => some("timestamp").map(|at| Actor {
                by: String::new(),
                at,
            }),
        };
        Front {
            title: some("title"),
            tags: strings("tags"),
            icon: some("icon"),
            concept: some("type"),
            status: some("status"),
            stale_after: some("stale_after"),
            about: Concept {
                description: yaml.text("description").to_string(),
                resource: yaml.text("resource").to_string(),
                generated,
                // A bare mapping is a one-element list (§5.2); `items` reads it as one.
                verified: yaml
                    .get("verified")
                    .map(|v| v.items().into_iter().map(actor).collect())
                    .unwrap_or_default(),
                sources: yaml
                    .get("sources")
                    .map(|sources| {
                        sources
                            .items()
                            .into_iter()
                            .map(|entry| Provenance {
                                id: entry.text("id").to_string(),
                                title: entry.text("title").to_string(),
                                resource: entry.text("resource").to_string(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            },
        }
    }
}

/// The trust tier a concept's `verified` events put it in (§5.3). The `human:` prefix
/// (§7) is the whole rule.
pub fn trust(verified: &[Actor]) -> &'static str {
    match verified {
        [] => "unverified",
        events if events.iter().any(|e| e.by.starts_with("human:")) => "human-reviewed",
        _ => "machine-confirmed",
    }
}

/// A concept is stale once `now` reaches its `stale_after` (§5.5).
pub fn is_stale(stale_after: &str, now: u64) -> bool {
    epoch(stale_after).is_some_and(|at| now >= at)
}

/// The trust and lifecycle signals a concept carries, in the order the legend lists them:
/// its trust tier, a status that is not the default, and staleness.
pub fn signals(front: &Front, now: u64) -> Vec<String> {
    let mut signals = vec![trust(&front.about.verified).to_string()];
    match front.status.as_deref().unwrap_or("stable") {
        "stable" => {}
        status => signals.push(status.to_string()),
    }
    if is_stale(front.stale_after.as_deref().unwrap_or(""), now) {
        signals.push("stale".to_string());
    }
    signals
}

/// The group a concept belongs to: the `type` it declares, and nothing derived. A
/// reserved file declares none because it is not a concept.
pub fn concept_type<'a>(path: &str, front: &'a Front) -> &'a str {
    if is_reserved(path) {
        return RESERVED;
    }
    match front.concept.as_deref() {
        Some(name) if !name.is_empty() => name,
        _ => UNTYPED,
    }
}

/// The reserved filenames, at any level of the bundle (§3.1).
pub fn is_reserved(path: &str) -> bool {
    matches!(crate::node::stem_of(path), "index" | "log")
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// An ISO 8601 datetime with an explicit UTC offset, as seconds since the epoch. Every
/// OKF timestamp is one; a date alone names a different instant in every timezone, so it
/// is refused rather than guessed at.
pub fn epoch(iso: &str) -> Option<u64> {
    let (date, rest) = iso.trim().split_once('T')?;
    let (y, m, d) = split3(date, '-')?;
    let at = rest.find(['Z', '+']).or_else(|| rest.rfind('-'))?;
    let (clock, offset) = (&rest[..at], &rest[at..]);
    let (hh, mm, ss) = split3(clock, ':')?;
    let seconds = days_from_civil(y as i64, m as i64, d as i64) * 86_400
        + (hh as i64) * 3600
        + (mm as i64) * 60
        + ss as i64;
    u64::try_from(seconds - offset_seconds(offset)?).ok()
}

/// Seconds to subtract to reach UTC: `Z` is none, `+01:00` is an hour ahead of it.
fn offset_seconds(offset: &str) -> Option<i64> {
    if offset == "Z" {
        return Some(0);
    }
    let sign = match offset.chars().next()? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    let (hours, minutes) = offset[1..].split_once(':')?;
    Some(sign * (hours.parse::<i64>().ok()? * 3600 + minutes.parse::<i64>().ok()? * 60))
}

fn split3(text: &str, sep: char) -> Option<(u32, u32, u32)> {
    let mut parts = text.split(sep);
    let mut next = || parts.next()?.parse::<u32>().ok();
    let three = (next()?, next()?, next()?);
    parts.next().is_none().then_some(three)
}

/// Days between the epoch and a civil date, by Howard Hinnant's algorithm — the one
/// piece of calendar arithmetic the standard library does not do.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let year_of_era = y - era * 400;
    let month = (m + 9) % 12;
    let day_of_year = (153 * month + 2) / 5 + d - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::fixture;
    use crate::yaml::parse;

    fn by(who: &str, at: &str) -> Actor {
        Actor {
            by: who.into(),
            at: at.into(),
        }
    }

    #[test]
    fn a_human_verifier_outranks_a_machine_one() {
        let at = "2026-06-25T09:00:00Z";
        assert_eq!(trust(&[]), "unverified");
        assert_eq!(
            trust(&[by("reference_agent/gemini-2.5-pro", at)]),
            "machine-confirmed"
        );
        assert_eq!(trust(&[by("human:ahormati", at)]), "human-reviewed");
        assert_eq!(
            trust(&[by("process:nightly", at), by("human:ahormati", at)]),
            "human-reviewed",
            "one human among the machines is enough"
        );
    }

    #[test]
    fn the_frontmatter_reads_into_the_concept_the_reader_shows() {
        let front = Front::read(&parse(
            "type: Attested Computation\ntitle: Revenue\ndescription: Sanctioned SQL.\n\
             resource: https://console.cloud.google.com/bigquery\ntags: [finance, revenue]\n\
             generated: { by: reference_agent/gemini-2.5-pro, at: 2026-06-30T14:00:00Z }\n\
             verified: { by: human:jsmith@acme, at: 2026-07-01T09:00:00Z }\n\
             sources:\n  - id: policy\n    resource: policies/revenue.md\n    title: Revenue policy\n",
        ));
        assert_eq!(front.concept.as_deref(), Some("Attested Computation"));
        assert_eq!(front.tags, ["finance", "revenue"]);
        assert_eq!(front.about.description, "Sanctioned SQL.");
        assert_eq!(
            front.about.resource,
            "https://console.cloud.google.com/bigquery"
        );
        assert_eq!(
            front.about.generated,
            Some(by("reference_agent/gemini-2.5-pro", "2026-06-30T14:00:00Z"))
        );
        assert_eq!(
            front.about.verified,
            [by("human:jsmith@acme", "2026-07-01T09:00:00Z")],
            "a bare mapping is a one-element list"
        );
        assert_eq!(
            front.about.sources,
            [Provenance {
                id: "policy".into(),
                title: "Revenue policy".into(),
                resource: "policies/revenue.md".into()
            }]
        );
    }

    #[test]
    fn a_v0_1_timestamp_stands_in_for_generated() {
        let front = Front::read(&parse(
            "type: Metric\ntimestamp: '2026-05-28T22:53:05+00:00'\ntags: single\n",
        ));
        assert_eq!(
            front.about.generated,
            Some(by("", "2026-05-28T22:53:05+00:00"))
        );
        assert_eq!(front.tags, ["single"], "a lone tag is a list of one");
        assert_eq!(Front::read(&parse("")), Front::default());
    }

    #[test]
    fn a_timestamp_needs_a_time_and_an_offset() {
        assert_eq!(epoch("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(epoch("2026-06-30T14:00:00Z"), Some(1_782_828_000));
        assert_eq!(
            epoch("2026-06-30T15:00:00+01:00"),
            epoch("2026-06-30T14:00:00Z"),
            "an offset is taken off"
        );
        assert_eq!(
            epoch("2026-06-30T13:00:00-01:00"),
            epoch("2026-06-30T14:00:00Z")
        );
        assert_eq!(epoch("2026-12-31"), None, "a date names no instant");
        assert_eq!(
            epoch("2026-12-31T00:00:00"),
            None,
            "and neither does a clock"
        );
        assert_eq!(epoch("whenever"), None);
    }

    #[test]
    fn staleness_is_a_comparison_and_nothing_else() {
        let now = epoch("2026-09-07T00:00:00Z").unwrap();
        assert!(is_stale("2026-09-06T23:59:59Z", now));
        assert!(!is_stale("2026-09-08T00:00:00Z", now));
        assert!(
            !is_stale("", now),
            "a concept that named no instant is fresh"
        );
        assert!(
            !is_stale("2026-12-31", now),
            "and so is one that named a day"
        );
    }

    #[test]
    fn a_concept_carries_its_tier_its_status_and_its_staleness() {
        let vault = fixture(&[
            (
                "metrics/revenue.md",
                "---\ntype: Metric\nstatus: stable\nverified: { by: human:a, at: 2026-06-25T09:00:00Z }\n---\n",
            ),
            (
                "metrics/old.md",
                "---\ntype: Metric\nstatus: deprecated\nstale_after: 2020-01-01T00:00:00Z\n---\n",
            ),
            ("index.md", "# Bundle\n"),
        ]);
        let signals = |path: &str| signals(&vault.note(path).unwrap().front, now());
        assert_eq!(signals("metrics/revenue.md"), ["human-reviewed"]);
        assert_eq!(
            signals("metrics/old.md"),
            ["unverified", "deprecated", "stale"]
        );
        let of = |path: &str| concept_type(path, &vault.note(path).unwrap().front).to_string();
        assert_eq!(of("metrics/old.md"), "Metric");
        assert_eq!(of("index.md"), RESERVED);
    }
}
