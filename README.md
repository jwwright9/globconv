# globconv

A small command-line tool that converts between shell glob patterns
(the `*.txt`, `file-?.log`, `[a-z]*` kind you use with `ls` or in a
`.gitignore`) and the regular expressions that describe the same set
of matches.

I wanted this because I kept hand-translating globs into regex when
wiring pattern matching into tools that only speak one or the other
(some config formats accept globs, some accept regex, and never the
one you already have written down).

## Usage

Build it with cargo, no other dependencies required:

```
cargo build --release
```

Convert a glob to its equivalent regex:

```
$ globconv to-regex '*.txt'
^.*\.txt$

$ globconv to-regex 'report-[0-9][0-9].csv'
^report-[0-9][0-9]\.csv$

$ globconv to-regex 'backup-[!0-9]*.tar.gz'
^backup-[^0-9].*\.tar\.gz$
```

Convert a regex back to a glob, when the regex is simple enough to
have one:

```
$ globconv to-glob '^.*\.rs$'
*.rs

$ globconv to-glob '^[a-z]+\.log$'
error: no glob equivalent for '+'
```

The `to-glob` direction is intentionally strict. Globs can express
"any characters," "any one character," and character classes, but
nothing else — no repetition counts, no groups, no alternation. If a
regex uses one of those, there is no glob that means the same thing,
and the tool says so instead of returning something approximate.

## What's supported

Glob syntax understood by `to-regex`:

- `*` — any run of characters, including zero
- `?` — any single character
- `[abc]`, `[a-z]`, `[!abc]` — character classes and negation
- `\x` — escape a character so it's matched literally

`**` is currently treated the same as `*`. Giving it the usual
"match across directory separators" meaning is on the list, see
below.

## Roadmap

- distinguish `**` from `*` for path-aware matching
- read patterns from stdin, one per line, for batch conversion
- a `--test <path>` flag to check a glob/regex against a sample string
- brace expansion (`{a,b,c}`)

## License

MIT, see LICENSE.
