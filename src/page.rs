//! The page the browser gets. It is written as a folder of parts under `src/web/` —
//! one per concern — and joined here at compile time, so the caller sees one function
//! and the browser still receives a single self-contained file. No build step, no CDN.
//!
//! The order below is the order the script runs in, and `tests/sim.mjs` reads this very
//! list to assemble the same script for its own run. A part that is not listed here is
//! neither served nor tested.

const SCRIPT: &str = concat!(
    include_str!("web/theme.js"),
    include_str!("web/graph.js"),
    include_str!("web/sim.js"),
    include_str!("web/render.js"),
    include_str!("web/input.js"),
    include_str!("web/markdown.js"),
    include_str!("web/reader.js"),
    include_str!("web/chrome.js"),
);

/// The page with this run's graph and token substituted in.
pub fn render(graph_json: &str, token: &str) -> String {
    include_str!("web/page.html")
        .replace("__STYLE__", include_str!("web/style.css"))
        .replace("__SCRIPT__", SCRIPT)
        .replace("__GRAPH__", graph_json)
        .replace("__TOKEN__", token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_placeholder_is_filled() {
        let page = render(r#"{"nodes":[]}"#, "abc123");
        for left in ["__STYLE__", "__SCRIPT__", "__GRAPH__", "__TOKEN__"] {
            assert!(!page.contains(left), "{left} survived into the page");
        }
        assert!(page.contains(r#"{"nodes":[]}"#), "the graph reaches the page");
        assert!(page.contains("abc123"), "the token reaches the page");
    }

    #[test]
    fn the_parts_are_joined_in_order() {
        let page = render("{}", "t");
        let at = |needle| page.find(needle).expect(needle);
        // One line from each part, in the order the script has to run in: a theme before
        // the canvas that paints with it, the model before the forces that move it.
        assert!(at("const THEMES") < at("const canvas"));
        assert!(at("const canvas") < at("const LINK_DISTANCE"));
        assert!(at("const LINK_DISTANCE") < at("function draw"));
        assert!(at("function draw") < at("function markdown"));
        assert!(at("function markdown") < at("function restart"));
    }
}
