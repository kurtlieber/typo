//! Syntax highlighting for code fences, with syntect and bat's grammars.
//! Grammars name TextMate scopes; each scope maps to a `Token` whose colour
//! comes from the Omarchy theme, the way Omarchy's own editor templates map
//! syntax to the terminal palette.

use std::ops::Range;
use std::sync::OnceLock;

use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};

/// What a run of code is, for colouring.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    Plain,
    Keyword,
    Control,
    Operator,
    Function,
    Special,
    Type,
    Namespace,
    Constant,
    Number,
    Character,
    Escape,
    String,
    Regexp,
    Comment,
    Parameter,
    Builtin,
    Member,
    Punctuation,
    Tag,
    Attribute,
    Heading,
}

impl Token {
    pub const ALL: [Token; 22] = [
        Token::Plain,
        Token::Keyword,
        Token::Control,
        Token::Operator,
        Token::Function,
        Token::Special,
        Token::Type,
        Token::Namespace,
        Token::Constant,
        Token::Number,
        Token::Character,
        Token::Escape,
        Token::String,
        Token::Regexp,
        Token::Comment,
        Token::Parameter,
        Token::Builtin,
        Token::Member,
        Token::Punctuation,
        Token::Tag,
        Token::Attribute,
        Token::Heading,
    ];

    /// The palette colour and whether it is italic, after Omarchy's Helix
    /// template (`default/themed/helix.toml.tpl`).
    pub fn style(self) -> (&'static str, bool) {
        match self {
            Token::Plain => ("foreground", false),
            Token::Keyword => ("color5", false),
            Token::Control => ("color5", true),
            Token::Operator => ("color6", false),
            Token::Function => ("color4", false),
            Token::Special => ("color5", false),
            Token::Type => ("color3", false),
            Token::Namespace => ("color3", true),
            Token::Constant | Token::Number | Token::Attribute => ("color3", false),
            Token::Character => ("color6", false),
            Token::Escape | Token::Regexp => ("color5", false),
            Token::String => ("color2", false),
            Token::Comment => ("color8", true),
            Token::Parameter => ("color5", true),
            Token::Builtin => ("color1", false),
            Token::Member | Token::Tag => ("color4", false),
            // Helix greys punctuation out (color8); CodeMirror, which Typora
            // uses, leaves it in the text colour, which reads better.
            Token::Punctuation => ("foreground", false),
            Token::Heading => ("color1", false),
        }
    }
}

/// Scope prefixes, most specific first within a family. A scope takes the
/// longest prefix it matches.
const SCOPES: &[(&str, Token)] = &[
    ("comment", Token::Comment),
    ("string.regexp", Token::Regexp),
    ("string", Token::String),
    ("constant.numeric", Token::Number),
    ("constant.character.escape", Token::Escape),
    ("constant.character", Token::Character),
    ("constant", Token::Constant),
    ("support.constant", Token::Constant),
    ("entity.name.constant", Token::Constant),
    ("keyword.operator.word", Token::Keyword),
    ("keyword.operator", Token::Operator),
    ("keyword.control", Token::Control),
    ("keyword", Token::Keyword),
    ("storage", Token::Keyword),
    ("entity.name.function.macro", Token::Special),
    ("support.macro", Token::Special),
    ("entity.name.function", Token::Function),
    ("support.function", Token::Function),
    ("variable.function", Token::Function),
    ("meta.annotation", Token::Special),
    ("meta.decorator", Token::Special),
    ("entity.name.namespace", Token::Namespace),
    ("entity.name.module", Token::Namespace),
    ("entity.name.tag", Token::Tag),
    ("entity.other.attribute-name", Token::Attribute),
    ("entity.name.section", Token::Heading),
    ("markup.heading", Token::Heading),
    ("markup.raw", Token::String),
    ("markup.underline.link", Token::Function),
    ("markup.inserted", Token::String),
    ("markup.deleted", Token::Builtin),
    ("entity.name.type", Token::Type),
    ("entity.name.class", Token::Type),
    ("entity.name.struct", Token::Type),
    ("entity.name.enum", Token::Type),
    ("entity.name.union", Token::Type),
    ("entity.name.trait", Token::Type),
    ("entity.name.interface", Token::Type),
    ("entity.other.inherited-class", Token::Type),
    ("support.type", Token::Type),
    ("support.class", Token::Type),
    ("entity.name", Token::Function),
    ("variable.parameter", Token::Parameter),
    ("variable.language", Token::Builtin),
    ("variable.other.member", Token::Member),
    ("variable.other.property", Token::Member),
    ("punctuation", Token::Punctuation),
];

fn token_of_scope(scope: &str) -> Option<Token> {
    SCOPES
        .iter()
        .filter(|(p, _)| scope == *p || scope.strip_prefix(p).is_some_and(|rest| rest.starts_with('.')))
        .max_by_key(|(p, _)| p.len())
        .map(|&(_, t)| t)
}

/// The token for a stack of scopes, outermost first. The innermost scope
/// that means something decides, except that punctuation inside a string
/// or comment (quotes, `//`) takes the colour of what it delimits.
fn token_of(stack: &[Scope]) -> Token {
    let mut punctuation = false;
    for scope in stack.iter().rev() {
        match token_of_scope(&scope.build_string()) {
            Some(Token::Punctuation) => punctuation = true,
            Some(t @ (Token::String | Token::Comment | Token::Regexp)) => return t,
            Some(_) if punctuation => return Token::Punctuation,
            Some(t) => return t,
            None => {}
        }
    }
    if punctuation { Token::Punctuation } else { Token::Plain }
}

fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(two_face::syntax::extra_newlines)
}

/// Names people write after the fence that the grammars don't know,
/// after Typora's language aliases.
const ALIASES: &[(&str, &str)] = &[
    ("shell", "bash"),
    ("console", "bash"),
    ("terminal", "bash"),
    ("zsh", "bash"),
    ("golang", "go"),
    ("c++", "cpp"),
    ("objc", "objective-c"),
    ("objectivec", "objective-c"),
    ("csharp", "c#"),
    ("fortran", "f90"),
    ("regex", "re"),
    ("js", "javascript"),
    ("jsx", "javascript"),
    ("node", "javascript"),
    ("ts", "typescript"),
    ("py", "python"),
    ("python3", "python"),
    ("rb", "ruby"),
    ("yml", "yaml"),
    ("text", "txt"),
    ("plain", "txt"),
    ("plaintext", "txt"),
    ("docker", "dockerfile"),
    ("ps1", "powershell"),
    ("vb", "vb.net"),
    ("tex", "latex"),
    ("make", "makefile"),
    ("protobuf", "proto"),
];

/// The grammar for a fence's info string: its first word, without a
/// `language-` prefix, by name or file extension.
pub fn syntax_for(info: &str) -> Option<&'static SyntaxReference> {
    let word = info.split_whitespace().next()?.to_lowercase();
    let word = word.trim_start_matches('.').trim_start_matches("language-").trim_start_matches("lang-");
    let word = ALIASES.iter().find(|(a, _)| *a == word).map_or(word, |&(_, to)| to);
    let set = syntaxes();
    set.find_syntax_by_token(word).or_else(|| set.syntaxes().iter().find(|s| s.name.eq_ignore_ascii_case(word)))
}

/// What the code language box suggests: common languages, each a name that
/// `syntax_for` resolves.
const LANGUAGES: &[&str] = &[
    "applescript", "asp", "awk", "bash", "bat", "c", "c#", "c++", "clojure", "cmake", "coffeescript",
    "cpp", "crystal", "csharp", "css", "csv", "d", "dart", "diff", "dockerfile", "dot", "elixir", "elm",
    "erlang", "f#", "fish", "fortran", "gitconfig", "glsl", "go", "graphql", "groovy", "haskell", "hcl",
    "html", "ini", "java", "javascript", "jinja2", "js", "json", "jsx", "julia", "kotlin", "latex",
    "less", "lisp", "lua", "makefile", "markdown", "matlab", "nginx", "nim", "nix", "objc", "ocaml",
    "pascal", "perl", "php", "powershell", "protobuf", "python", "r", "racket", "regex", "rst", "ruby",
    "rust", "sass", "scala", "scss", "shell", "sql", "svelte", "swift", "tcl", "terraform",
    "tex", "toml", "ts", "tsx", "typescript", "verilog", "vhdl", "vim", "vue", "xml", "yaml", "zig",
];

/// The code language box's suggestions: the names above that resolve.
pub fn languages() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| LANGUAGES.iter().copied().filter(|n| syntax_for(n).is_some()).collect())
}

/// A highlighted code block: per line, the runs (byte ranges within the
/// line, without its newline) and their tokens. It keeps each line's parser
/// state so an edit re-parses from the changed line only until the state
/// settles back to what it was.
#[derive(Default)]
pub struct Highlight {
    syntax: Option<&'static str>,
    lines: Vec<Line>,
}

#[derive(Clone)]
struct Line {
    text: String,
    /// State after this line.
    state: (ParseState, ScopeStack),
    runs: Vec<(Range<usize>, Token)>,
}

impl Highlight {
    /// Update for `text` in the language named by `info`. Returns the index
    /// of the first line whose runs may have changed.
    pub fn update(&mut self, info: &str, text: &str) -> usize {
        let Some(syntax) = syntax_for(info) else {
            self.syntax = None;
            self.lines.clear();
            return 0;
        };
        if self.syntax != Some(syntax.name.as_str()) {
            self.syntax = Some(syntax.name.as_str());
            self.lines.clear();
        }
        let set = syntaxes();
        let new: Vec<&str> = text.split('\n').collect();
        let old = std::mem::take(&mut self.lines);
        // The edit lies between an unchanged prefix and an unchanged suffix.
        let first = old.iter().zip(&new).take_while(|(l, n)| l.text == **n).count();
        let room = old.len().min(new.len()) - first;
        let suffix = (1..=room).take_while(|k| old[old.len() - k].text == new[new.len() - k]).count();
        let mut state = match first {
            0 => (ParseState::new(syntax), ScopeStack::new()),
            i => old[i - 1].state.clone(),
        };
        self.lines = old[..first].to_vec();
        for (i, line) in new.iter().enumerate().skip(first) {
            // In the suffix, a line entered in the same state as before ends
            // the same way, and so does everything after it.
            if i > first && i >= new.len() - suffix {
                let j = i + old.len() - new.len();
                if old[j - 1].state == state {
                    self.lines.extend_from_slice(&old[j..]);
                    break;
                }
            }
            let (runs, next) = highlight_line(set, line, state);
            state = next.clone();
            self.lines.push(Line { text: line.to_string(), state: next, runs });
        }
        first
    }

    /// The runs of line `i`.
    pub fn runs(&self, i: usize) -> &[(Range<usize>, Token)] {
        self.lines.get(i).map_or(&[], |l| &l.runs)
    }

    /// Qt's state for line `i`. Qt re-highlights the next line when it
    /// changes, so it hashes what the next line looks like.
    pub fn state_hash(&self, i: usize) -> i32 {
        use std::hash::{Hash, Hasher};
        let Some(l) = self.lines.get(i) else { return -1 };
        let mut h = std::collections::hash_map::DefaultHasher::new();
        l.state.1.as_slice().hash(&mut h);
        self.runs(i + 1).hash(&mut h);
        (h.finish() & 0x3fff_ffff) as i32
    }
}

/// Parser states for Source Code Mode's fences, where Qt keeps one small
/// number a line: each state is kept once and named by its place (from
/// 1), so a line whose state comes out as before stops Qt re-colouring
/// the lines after it.
#[derive(Default)]
pub struct States {
    all: Vec<(ParseState, ScopeStack)>,
    by_scopes: std::collections::HashMap<u64, Vec<u16>>,
}

impl States {
    /// What a fence in the language `info` names starts in; 0 for a
    /// language without a grammar.
    pub fn start(&mut self, info: &str) -> u16 {
        syntax_for(info).map_or(0, |s| self.number((ParseState::new(s), ScopeStack::new())))
    }

    /// Line `line` after state `n`: its runs and the state after it. None
    /// when `n` isn't a state here.
    pub fn line(&mut self, n: u16, line: &str) -> Option<(Vec<(Range<usize>, Token)>, u16)> {
        let state = self.all.get(usize::from(n).checked_sub(1)?)?.clone();
        let (runs, next) = highlight_line(syntaxes(), line, state);
        Some((runs, self.number(next)))
    }

    pub fn clear(&mut self) {
        self.all.clear();
        self.by_scopes.clear();
    }

    /// The state's number; 0 once there are too many to number.
    fn number(&mut self, state: (ParseState, ScopeStack)) -> u16 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        state.1.as_slice().hash(&mut h);
        let same = self.by_scopes.entry(h.finish()).or_default();
        if let Some(&n) = same.iter().find(|&&n| self.all[usize::from(n) - 1] == state) {
            return n;
        }
        if self.all.len() >= usize::from(u16::MAX) {
            return 0;
        }
        self.all.push(state);
        let n = self.all.len() as u16;
        same.push(n);
        n
    }
}

fn highlight_line(
    set: &SyntaxSet,
    line: &str,
    (mut parse, mut stack): (ParseState, ScopeStack),
) -> (Vec<(Range<usize>, Token)>, (ParseState, ScopeStack)) {
    // The grammars expect each line with its newline.
    let with_nl = format!("{line}\n");
    let ops = parse.parse_line(&with_nl, set).unwrap_or_default();
    let mut runs: Vec<(Range<usize>, Token)> = Vec::new();
    let mut at = 0;
    let mut push = |range: Range<usize>, stack: &ScopeStack| {
        let range = range.start.min(line.len())..range.end.min(line.len());
        if range.is_empty() {
            return;
        }
        let t = token_of(stack.as_slice());
        match runs.last_mut() {
            Some((r, last)) if *last == t && r.end == range.start => r.end = range.end,
            _ => runs.push((range, t)),
        }
    };
    for (pos, op) in ops {
        push(at..pos, &stack);
        at = pos;
        let _ = stack.apply(&op);
    }
    push(at..with_nl.len(), &stack);
    runs.retain(|(_, t)| *t != Token::Plain);
    (runs, (parse, stack))
}

#[cfg(test)]
mod tests {
    #[test]
    fn states_number_alike() {
        let mut s = super::States::default();
        let js = s.start("js");
        assert!(js > 0 && s.start("javascript") == js && s.start("nosuchlanguage") == 0);
        let (runs, after) = s.line(js, "const x = 1;").unwrap();
        assert!(runs.iter().any(|(_, t)| *t == super::Token::Keyword));
        assert_eq!(s.line(after, "let y = 2;").unwrap().1, after);
        let (_, open) = s.line(after, "/* open").unwrap();
        assert_ne!(open, after);
        assert_eq!(s.line(open, "still */").unwrap().1, after);
        assert!(s.line(999, "x").is_none() && s.line(0, "x").is_none());
    }

    use super::*;

    fn show(info: &str, text: &str) -> Vec<String> {
        let mut h = Highlight::default();
        h.update(info, text);
        text.split('\n')
            .enumerate()
            .map(|(i, line)| {
                h.runs(i).iter().map(|(r, t)| format!("{}={:?}", &line[r.clone()], t)).collect::<Vec<_>>().join(" ")
            })
            .collect()
    }

    #[test]
    fn finds_languages_by_name_alias_and_extension() {
        assert_eq!(syntax_for("rust").unwrap().name, "Rust");
        assert_eq!(syntax_for("rs {.numberLines}").unwrap().name, "Rust");
        assert_eq!(syntax_for("JS").unwrap().name, "JavaScript");
        assert_eq!(syntax_for("shell").unwrap().name, "Bourne Again Shell (bash)");
        assert_eq!(syntax_for("c++").unwrap().name, "C++");
        assert_eq!(syntax_for("language-python").unwrap().name, "Python");
        assert!(syntax_for("toml").is_some());
        assert!(syntax_for("typescript").is_some());
        assert!(syntax_for("").is_none());
        assert!(syntax_for("no-such-language").is_none());
        let missing: Vec<_> = LANGUAGES.iter().filter(|n| syntax_for(n).is_none()).collect();
        assert!(missing.is_empty(), "no grammar for {missing:?}");
    }

    #[test]
    fn tokens_follow_scopes() {
        let lines = show("rust", "fn main() {\n    let s = \"hi\"; // note\n}");
        assert!(lines[0].starts_with("fn=Keyword main=Function"), "{}", lines[0]);
        assert!(lines[1].contains("\"hi\"=String"), "{}", lines[1]);
        assert!(lines[1].contains("// note=Comment"), "{}", lines[1]);
    }

    #[test]
    fn an_edit_carries_state_to_later_lines() {
        let mut h = Highlight::default();
        h.update("c", "int a;\nint b;\nint c;");
        assert_eq!(h.runs(2).first().map(|r| r.1), Some(Token::Keyword));
        let first = h.update("c", "/* int a;\nint b;\nint c;");
        assert_eq!(first, 0);
        // Every line is now inside the open comment.
        for i in 0..3 {
            assert!(h.runs(i).iter().all(|(_, t)| *t == Token::Comment), "line {i}: {:?}", h.runs(i));
        }
        // Closing it restores the last line.
        h.update("c", "/* int a; */\nint b;\nint c;");
        assert!(h.runs(2).iter().any(|(_, t)| *t != Token::Comment));
    }
}
