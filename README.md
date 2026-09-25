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
^[^/]*\.txt$

$ globconv to-regex 'report-[0-9][0-9].csv'
^report-[0-9][0-9]\.csv$

$ globconv to-regex 'backup-[!0-9]*.tar.gz'
^backup-[^0-9][^/]*\.tar\.gz$

$ globconv to-regex 'logs/**/*.log'
^logs/.*/[^/]*\.log$

$ globconv to-regex 'file.{txt,log,bak}'
^(?:file\.txt|file\.log|file\.bak)$
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

Check a glob pattern against a sample string directly, without going
through the regex conversion:

```
$ globconv test '*.txt' report.txt
match

$ globconv test 'logs/**/*.log' logs/2024/01/a.log
match

$ globconv test '*.log' logs/a.log
no match

$ globconv test 'archive.{tar.gz,zip}' archive.zip
match
```

`test` exits with status 0 on a match and 1 otherwise, so it works as
a condition in scripts. It matches directly against the glob syntax
rather than compiling to regex first — there's no regex engine in
the standard library to run a compiled pattern against.

## Batch mode

Leave off the pattern argument and it reads patterns from stdin, one
per line, converting each and printing the result on its own line of
output:

```
$ printf '*.txt\nlogs/**/*.log\n' | globconv to-regex
^[^/]*\.txt$
^logs/.*/[^/]*\.log$
```

If any line fails to convert (relevant to `to-glob`), the error goes
to stderr with the offending pattern, the rest of the batch keeps
going, and the process exits non-zero once stdin is exhausted.

## What's supported

Glob syntax understood by `to-regex`:

- `*` — any run of characters except `/`, including zero
- `**` — any run of characters, including `/`, for matching across
  directory separators (`logs/**/*.log` matches `logs/2024/01/a.log`)
- `?` — any single character
- `[abc]`, `[a-z]`, `[!abc]` — character classes and negation
- `{a,b,c}` — brace alternatives, expanded the way a shell would
  before the rest of the pattern is interpreted (so `{a,{b,c}}`
  nests, and a brace with no comma in it, like `{1}`, is left as a
  literal instead of being treated as a group)
- `\x` — escape a character so it's matched literally

`to-glob` recognizes the regex these compile to (`[^/]*` and `.*`)
and folds them back into `*` and `**` respectively. It does not fold
an alternation like `(?:a|b)` back into `{a,b}` — same as any other
group or alternation, there's no glob equivalent as far as `to-glob`
is concerned, only the reverse direction expands braces.

## Roadmap

- integration tests that exercise the built binary directly

## License

MIT, see LICENSE.
