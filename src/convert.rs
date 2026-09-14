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
}
