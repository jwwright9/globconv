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
/// Supported syntax: `*` (any run of characters, including none),
/// `?` (any single character), `[abc]` / `[!abc]` character classes,
/// and `\` to escape the next character literally. `**` is treated
/// the same as `*` — this tool does not give it special recursive
/// meaning yet.
pub fn glob_to_regex(pattern: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::from("^");
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' => out.push_str(".*"),
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
/// `.` and `.*`, character classes `[...]` / `[^...]`, escaped
/// literals, and plain characters. Quantifiers, groups, alternation
/// and anchors other than a leading `^` / trailing `$` are rejected
/// rather than approximated.
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
                    out.push('*');
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
        assert_eq!(glob_to_regex("*.txt"), "^.*\\.txt$");
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
    fn roundtrip_class() {
        let re = glob_to_regex("[a-z].log");
        assert_eq!(regex_to_glob(&re).unwrap(), "[a-z]?log");
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
