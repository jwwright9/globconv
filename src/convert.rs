// Conversion between shell-style glob patterns and the regex dialect
// understood by most standard-library-free regex engines (POSIX-ish,
// no lookaround, no backreferences). Both directions are best-effort:
// glob -> regex is total, regex -> glob rejects anything that doesn't
// have a clean glob equivalent instead of guessing.

fn regex_escape(c: char) -> String {
    match c {
        '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\' => {
            let mut s = String::with_capacity(2);
            s.push('\\');
            s.push(c);
            s
        }
        _ => c.to_string(),
    }
}

/// Translate a glob pattern into an anchored regex.
///
/// Supported syntax: `*` (any run of characters except `/`), `**` (any
/// run of characters, including `/`, for matching across path
/// segments), `?` (any single character), `[abc]` / `[!abc]` character
/// classes, and `\` to escape the next character literally.
pub fn glob_to_regex(pattern: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::from("^");
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' => {
                let mut j = i;
                while j < chars.len() && chars[j] == '*' {
                    j += 1;
                }
                if j - i >= 2 {
                    out.push_str(".*");
                } else {
                    out.push_str("[^/]*");
                }
                i = j - 1;
            }
            '?' => out.push('.'),
            '[' => {
                let mut j = i + 1;
                let mut negate = false;
                if j < chars.len() && (chars[j] == '!' || chars[j] == '^') {
                    negate = true;
                    j += 1;
                }
                let class_start = j;
                if j < chars.len() && chars[j] == ']' {
                    j += 1; // a ']' right after the opening bracket is literal
                }
                while j < chars.len() && chars[j] != ']' {
                    j += 1;
                }
                if j < chars.len() {
                    out.push('[');
                    if negate {
                        out.push('^');
                    }
                    for &cc in &chars[class_start..j] {
                        if cc == '\\' {
                            out.push_str("\\\\");
                        } else {
                            out.push(cc);
                        }
                    }
                    out.push(']');
                    i = j;
                } else {
                    // unterminated class: treat the bracket as a literal
                    out.push_str("\\[");
                }
            }
            '\\' => {
                if i + 1 < chars.len() {
                    i += 1;
                    out.push_str(&regex_escape(chars[i]));
                } else {
                    out.push_str("\\\\");
                }
            }
            c => out.push_str(&regex_escape(c)),
        }
        i += 1;
    }
    out.push('$');
    out
}

/// Translate a regex back into a glob pattern, when possible.
///
/// Only the subset of regex syntax that glob can express is accepted:
/// `.` and `.*` (mapped to `**`, since a bare `*` in glob already
/// excludes `/`), `[^/]*` (mapped back to `*`), character classes
/// `[...]` / `[^...]`, escaped literals, and plain characters.
/// Quantifiers, groups, alternation and anchors other than a leading
/// `^` / trailing `$` are rejected rather than approximated.
pub fn regex_to_glob(pattern: &str) -> Result<String, String> {
    let mut p = pattern;
    if let Some(stripped) = p.strip_prefix('^') {
        p = stripped;
    }
    if let Some(stripped) = p.strip_suffix('$') {
        p = stripped;
    }
    let chars: Vec<char> = p.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '.' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    out.push_str("**");
                    i += 1;
                } else {
                    out.push('?');
                }
            }
            '\\' => {
                if i + 1 >= chars.len() {
                    return Err("trailing backslash".to_string());
                }
                i += 1;
                let esc = chars[i];
                if esc == '*' || esc == '?' || esc == '[' || esc == '\\' {
                    out.push('\\');
                }
                out.push(esc);
            }
            '[' => {
                let mut j = i + 1;
                let mut negate = false;
                if j < chars.len() && chars[j] == '^' {
                    negate = true;
                    j += 1;
                }
                let start = j;
                while j < chars.len() && chars[j] != ']' {
                    j += 1;
                }
                if j >= chars.len() {
                    return Err("unterminated character class".to_string());
                }
                let followed_by_star = j + 1 < chars.len() && chars[j + 1] == '*';
                if negate && j - start == 1 && chars[start] == '/' && followed_by_star {
                    // [^/]* is what *-in-a-glob compiles to; fold it back.
                    out.push('*');
                    i = j + 1;
                } else if followed_by_star {
                    return Err("no glob equivalent for a quantified character class".to_string());
                } else {
                    out.push('[');
                    if negate {
                        out.push('!');
                    }
                    for &cc in &chars[start..j] {
                        out.push(cc);
                    }
                    out.push(']');
                    i = j;
                }
            }
            '*' | '+' | '?' | '(' | ')' | '{' | '}' | '|' | '^' | '$' => {
                return Err(format!("no glob equivalent for '{}'", chars[i]));
            }
            c => {
                if c == '*' || c == '?' || c == '[' {
                    out.push('\\');
                }
                out.push(c);
            }
        }
        i += 1;
    }
    Ok(out)
}

enum GlobToken {
    Star,
    GlobStar,
    AnyChar,
    Class { negate: bool, items: String },
    Literal(char),
}

fn parse_glob(pattern: &str) -> Vec<GlobToken> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' => {
                let mut j = i;
                while j < chars.len() && chars[j] == '*' {
                    j += 1;
                }
                if j - i >= 2 {
                    tokens.push(GlobToken::GlobStar);
                } else {
                    tokens.push(GlobToken::Star);
                }
                i = j;
                continue;
            }
            '?' => tokens.push(GlobToken::AnyChar),
            '[' => {
                let mut j = i + 1;
                let mut negate = false;
                if j < chars.len() && (chars[j] == '!' || chars[j] == '^') {
                    negate = true;
                    j += 1;
                }
                let class_start = j;
                if j < chars.len() && chars[j] == ']' {
                    j += 1;
                }
                while j < chars.len() && chars[j] != ']' {
                    j += 1;
                }
                if j < chars.len() {
                    tokens.push(GlobToken::Class {
                        negate,
                        items: chars[class_start..j].iter().collect(),
                    });
                    i = j;
                } else {
                    // unterminated class: treat the bracket as a literal,
                    // matching how glob_to_regex handles the same case.
                    tokens.push(GlobToken::Literal('['));
                }
            }
            '\\' => {
                if i + 1 < chars.len() {
                    i += 1;
                    tokens.push(GlobToken::Literal(chars[i]));
                } else {
                    tokens.push(GlobToken::Literal('\\'));
                }
            }
            c => tokens.push(GlobToken::Literal(c)),
        }
        i += 1;
    }
    tokens
}

fn class_matches(items: &str, c: char) -> bool {
    let chars: Vec<char> = items.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 2 < chars.len() && chars[i + 1] == '-' {
            if c >= chars[i] && c <= chars[i + 2] {
                return true;
            }
            i += 3;
        } else {
            if chars[i] == c {
                return true;
            }
            i += 1;
        }
    }
    false
}

fn char_matches(token: &GlobToken, c: char) -> bool {
    match token {
        GlobToken::AnyChar => true,
        GlobToken::Literal(l) => *l == c,
        GlobToken::Class { negate, items } => class_matches(items, c) != *negate,
        GlobToken::Star | GlobToken::GlobStar => unreachable!(),
    }
}

fn star_accepts(token: &GlobToken, c: char) -> bool {
    match token {
        GlobToken::Star => c != '/',
        GlobToken::GlobStar => true,
        _ => unreachable!(),
    }
}

fn is_star(token: &GlobToken) -> bool {
    matches!(token, GlobToken::Star | GlobToken::GlobStar)
}

fn match_tokens(tokens: &[GlobToken], text: &[char]) -> bool {
    let mut ti = 0;
    let mut si = 0;
    let mut star_ti: Option<usize> = None;
    let mut star_si = 0;

    while si < text.len() {
        if ti < tokens.len() && !is_star(&tokens[ti]) && char_matches(&tokens[ti], text[si]) {
            ti += 1;
            si += 1;
        } else if ti < tokens.len() && is_star(&tokens[ti]) {
            star_ti = Some(ti);
            star_si = si;
            ti += 1;
        } else if let Some(sti) = star_ti {
            // the star gives up one more character to the wildcard and we
            // retry matching from just after it
            if !star_accepts(&tokens[sti], text[star_si]) {
                return false;
            }
            star_si += 1;
            si = star_si;
            ti = sti + 1;
        } else {
            return false;
        }
    }

    while ti < tokens.len() && is_star(&tokens[ti]) {
        ti += 1;
    }
    ti == tokens.len()
}

/// Check whether `candidate` matches the glob `pattern`, using the same
/// syntax `glob_to_regex` understands (`*`, `**`, `?`, `[...]`, `\x`).
///
/// This matches directly against the parsed glob rather than compiling to
/// a regex first, since there is no regex engine in the standard library
/// to run the compiled pattern against.
pub fn glob_match(pattern: &str, candidate: &str) -> bool {
    let tokens = parse_glob(pattern);
    let text: Vec<char> = candidate.chars().collect();
    match_tokens(&tokens, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_star() {
        assert_eq!(glob_to_regex("*.txt"), "^[^/]*\\.txt$");
    }

    #[test]
    fn globstar_crosses_slash() {
        assert_eq!(glob_to_regex("**/*.log"), "^.*/[^/]*\\.log$");
    }

    #[test]
    fn triple_star_is_globstar() {
        assert_eq!(glob_to_regex("***"), "^.*$");
    }

    #[test]
    fn question_mark() {
        assert_eq!(glob_to_regex("a?c"), "^a.c$");
    }

    #[test]
    fn char_class() {
        assert_eq!(glob_to_regex("[!abc]"), "^[^abc]$");
    }

    #[test]
    fn escaped_star() {
        assert_eq!(glob_to_regex("\\*"), "^\\*$");
    }

    #[test]
    fn roundtrip_star() {
        let re = glob_to_regex("*.rs");
        assert_eq!(regex_to_glob(&re).unwrap(), "*.rs");
    }

    #[test]
    fn roundtrip_globstar() {
        let re = glob_to_regex("a/**/b.txt");
        assert_eq!(regex_to_glob(&re).unwrap(), "a/**/b.txt");
    }

    #[test]
    fn roundtrip_class() {
        let re = glob_to_regex("[a-z].log");
        assert_eq!(regex_to_glob(&re).unwrap(), "[a-z].log");
    }

    #[test]
    fn rejects_quantified_class() {
        assert!(regex_to_glob("^[abc]*$").is_err());
    }

    #[test]
    fn rejects_quantifiers() {
        assert!(regex_to_glob("^a+$").is_err());
    }

    #[test]
    fn rejects_alternation() {
        assert!(regex_to_glob("^(a|b)$").is_err());
    }

    #[test]
    fn match_simple_star() {
        assert!(glob_match("*.txt", "report.txt"));
        assert!(!glob_match("*.txt", "report.csv"));
    }

    #[test]
    fn match_star_excludes_slash() {
        assert!(!glob_match("*.log", "logs/a.log"));
    }

    #[test]
    fn match_globstar_crosses_slash() {
        assert!(glob_match("logs/**/*.log", "logs/2024/01/a.log"));
        assert!(glob_match("logs/**/*.log", "logs/a.log"));
    }

    #[test]
    fn match_question_mark() {
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "ac"));
    }

    #[test]
    fn match_char_class() {
        assert!(glob_match("report-[0-9][0-9].csv", "report-42.csv"));
        assert!(!glob_match("report-[0-9][0-9].csv", "report-4a.csv"));
    }

    #[test]
    fn match_negated_class() {
        assert!(glob_match("backup-[!0-9]*.tar.gz", "backup-x.tar.gz"));
        assert!(!glob_match("backup-[!0-9]*.tar.gz", "backup-9.tar.gz"));
    }

    #[test]
    fn match_escaped_literal() {
        assert!(glob_match("\\*.txt", "*.txt"));
        assert!(!glob_match("\\*.txt", "a.txt"));
    }

    #[test]
    fn match_requires_full_string() {
        assert!(!glob_match("*.txt", "report.txt.bak"));
        assert!(!glob_match("report", "report.txt"));
    }
}
