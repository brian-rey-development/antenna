//! The lines of the keys and the table headers in the text of a manifest. The `toml` crate gives
//! no positions, so the checks find the lines in the text.

/// The place of a table in a manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Scope {
    /// A table of the package, for example `[dependencies]`.
    Package,
    /// A table of a target, for example `[target.'cfg(windows)'.dependencies]`.
    Target,
}

/// Returns the index of the line that declares the key `name` in a table, as a key of the table
/// or as the header `[<table>.<name>]`.
pub(super) fn line_of(text: &str, scope: Scope, table: &str, name: &str) -> Option<usize> {
    let own_header = format!("{table}.{name}");
    let mut header = "";
    text.lines().position(|line| {
        if let Some(inner) = header_of(line) {
            header = inner;
            return is_table(header, scope, &own_header);
        }
        is_table(header, scope, table) && declares(line.trim(), name)
    })
}

/// Returns the index of the first header of a table or of one of its subtables, for example
/// `[lints]` or `[lints.clippy]` for the table `lints`.
pub(super) fn header_line(text: &str, table: &str) -> Option<usize> {
    text.lines().position(|line| {
        header_of(line).is_some_and(|header| {
            header == table
                || header
                    .strip_prefix(table)
                    .is_some_and(|rest| rest.starts_with('.'))
        })
    })
}

/// Returns the name of a table header line, for example `lints.clippy` for `[lints.clippy]`.
fn header_of(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .map(str::trim)
}

fn is_table(header: &str, scope: Scope, table: &str) -> bool {
    match scope {
        Scope::Package => header == table,
        Scope::Target => {
            header.starts_with("target.")
                && header
                    .strip_suffix(table)
                    .is_some_and(|rest| rest.ends_with('.'))
        }
    }
}

fn declares(line: &str, name: &str) -> bool {
    line.strip_prefix(name)
        .is_some_and(|rest| rest.trim_start().starts_with(['=', '.']))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_line_finds_subtable_when_table_header_missing() {
        let text = "[package]\nname = \"antenna-text\"\n\n[lints.clippy]\n";

        let line = header_line(text, "lints");

        assert_eq!(line, Some(3));
    }
}
