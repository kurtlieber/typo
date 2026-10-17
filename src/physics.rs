//! The `physics` package, which MathJax loads under Typora's "Enable
//! physics package": its commands written out as the plain LaTeX they
//! draw, before a formula is converted. Braces and brackets sized to what
//! they hold (`\qty`, `\abs`, `\norm`, `\eval`, `\order`, `\comm`), vectors
//! (`\vb`, `\va`, `\vu`, `\grad`, `\div`, `\curl`, `\laplacian`), operators
//! (`\tr`, `\Re`, `\Im`…), quick quad text (`\qq`, `\qand`…), derivatives
//! (`\dd`, `\dv`, `\pdv`, `\var`, `\fdv`), Dirac notation (`\bra`, `\ket`,
//! `\braket`, `\ketbra`, `\expval`, `\mel`) and matrices (`\mqty`,
//! `\pmqty`…, `\imat`, `\zmat`, `\dmat`, `\admat`). It redefines `\div`,
//! `\Re` and `\Im`, which is why it is a preference. What comes out holds
//! none of these, so expanding twice changes nothing.

/// The words quick quad text sets between quads: `\qif` is " if ".
const QUAD_WORDS: [&str; 19] = [
    "if", "then", "else", "otherwise", "unless", "given", "using", "assume", "since", "let", "for", "all", "even",
    "odd", "integer", "and", "or", "as", "in",
];

/// `tex` with the physics package's commands written out.
pub fn expand(tex: &str) -> String {
    let mut out = String::with_capacity(tex.len() + 16);
    let mut s = Scan { tex, at: 0 };
    while s.at < tex.len() {
        let start = s.at;
        if s.byte() != Some(b'\\') {
            s.at = tex[start..].find('\\').map_or(tex.len(), |i| start + i);
            out.push_str(&tex[start..s.at]);
            continue;
        }
        let name = s.word();
        let after = s.at;
        match command(name, &mut s) {
            Some(text) => out.push_str(&text),
            None => {
                // Not the package's, or a control symbol such as `\\`.
                s.at = if name.is_empty() { tex[after..].chars().next().map_or(after, |c| after + c.len_utf8()) } else { after };
                out.push_str(&tex[start..s.at]);
            }
        }
    }
    out
}

/// What command `name` (its arguments next in `s`) draws, if it is the
/// package's and its arguments are there.
fn command(name: &str, s: &mut Scan) -> Option<String> {
    let text = match name {
        "quantity" | "qty" => {
            let star = s.star();
            let (open, body) = s.delimited(b"([{|")?;
            fence(open, &expand(body), star)
        }
        "pqty" => fence(b'(', &s.arg()?, false),
        "bqty" => fence(b'[', &s.arg()?, false),
        "vqty" => fence(b'|', &s.arg()?, false),
        "Bqty" => fence(b'{', &s.arg()?, false),
        "absolutevalue" | "abs" => {
            let star = s.star();
            sized("|", &[s.inner()?], "|", star)
        }
        "norm" => {
            let star = s.star();
            sized("\\|", &[s.inner()?], "\\|", star)
        }
        "evaluated" | "eval" => {
            let (open, body) = s.delimited(b"([{")?;
            let open = match open {
                b'(' => "(",
                b'[' => "[",
                _ => ".",
            };
            format!("\\left{open} {} \\right|", expand(body))
        }
        "order" => format!("\\mathcal{{O}}{}", fence(b'(', &s.inner()?, false)),
        "commutator" | "comm" => format!("\\left[ {} , {} \\right]", s.arg()?, s.arg()?),
        "anticommutator" | "acomm" | "poissonbracket" | "pb" => format!("\\left\\{{ {} , {} \\right\\}}", s.arg()?, s.arg()?),
        "vectorbold" | "vb" => {
            let star = s.star();
            bold(&s.arg()?, star)
        }
        "vectorarrow" | "va" => {
            let star = s.star();
            format!("\\vec{{{}}}", bold(&s.arg()?, star))
        }
        "vectorunit" | "vu" => {
            let star = s.star();
            format!("\\hat{{{}}}", bold(&s.arg()?, star))
        }
        "dotproduct" | "vdot" => "\\cdot ".to_string(),
        "crossproduct" | "cross" | "cp" => "\\times ".to_string(),
        "gradient" | "grad" => format!("\\nabla {}", operand(s)),
        "divergence" | "div" => format!("\\nabla\\cdot {}", operand(s)),
        "curl" => format!("\\nabla\\times {}", operand(s)),
        "laplacian" => format!("\\nabla^2 {}", operand(s)),
        "tr" | "Tr" | "rank" | "erf" | "Res" | "Re" | "Im" => format!("\\operatorname{{{name}}}{}", operand(s)),
        "principalvalue" | "pv" => format!("\\mathcal{{P}}{}", operand(s)),
        "PV" => format!("\\operatorname{{P.V.}}{}", operand(s)),
        "qq" => format!("\\quad\\text{{{}}}\\quad ", s.group()?),
        "qcomma" | "qc" => ",\\quad ".to_string(),
        "qcc" => "\\quad\\text{c.c.}\\quad ".to_string(),
        _ if name.strip_prefix('q').is_some_and(|w| QUAD_WORDS.contains(&w)) => {
            format!("\\quad\\text{{{}}}\\quad ", &name[1..])
        }
        "differential" | "dd" => {
            let head = format!("\\mathrm{{d}}{}", power(s.optional().map(expand)));
            match s.delimited(b"({") {
                Some((b'(', body)) => format!("{head}{}", fence(b'(', &expand(body), false)),
                Some((_, body)) => format!("{head} {}", expand(body)),
                None => format!("{head} "),
            }
        }
        "variation" | "var" => match s.delimited(b"({") {
            Some((b'(', body)) => format!("\\delta{}", fence(b'(', &expand(body), false)),
            Some((_, body)) => format!("\\delta {}", expand(body)),
            None => "\\delta ".to_string(),
        },
        "derivative" | "dv" => derivative("\\mathrm{d}", 1, s)?,
        "partialderivative" | "pderivative" | "pdv" => derivative("\\partial", 2, s)?,
        "functionalderivative" | "fdv" => derivative("\\delta", 1, s)?,
        "bra" => {
            let star = s.star();
            sized("\\langle", &[s.arg()?], "|", star)
        }
        "ket" => {
            let star = s.star();
            sized("|", &[s.arg()?], "\\rangle", star)
        }
        "innerproduct" | "braket" | "ip" => {
            let star = s.star();
            let a = s.arg()?;
            let b = s.group().map(expand).unwrap_or_else(|| a.clone());
            sized("\\langle", &[a, b], "\\rangle", star)
        }
        "outerproduct" | "dyad" | "ketbra" | "op" => {
            let star = s.star();
            let a = s.arg()?;
            let b = s.group().map(expand).unwrap_or_else(|| a.clone());
            format!("{} {}", sized("|", &[a], "\\rangle", star), sized("\\langle", &[b], "|", star))
        }
        "expectationvalue" | "expval" | "ev" => {
            let star = s.star();
            let a = s.arg()?;
            match s.group().map(expand) {
                Some(psi) => sized("\\langle", &[psi.clone(), a, psi], "\\rangle", star),
                None => sized("\\langle", &[a], "\\rangle", star),
            }
        }
        "matrixelement" | "matrixel" | "mel" => {
            let star = s.star();
            sized("\\langle", &[s.arg()?, s.arg()?, s.arg()?], "\\rangle", star)
        }
        "matrixquantity" | "mqty" | "smallmatrixquantity" | "smqty" => {
            s.star();
            let (open, body) = s.delimited(b"([{|")?;
            let env = match open {
                b'(' => "pmatrix",
                b'[' => "bmatrix",
                b'|' => "vmatrix",
                _ => "matrix",
            };
            matrix(env, &expand(body))
        }
        "pmqty" | "Pmqty" => matrix("pmatrix", &expand(s.group()?)),
        "bmqty" => matrix("bmatrix", &expand(s.group()?)),
        "vmqty" => matrix("vmatrix", &expand(s.group()?)),
        "identitymatrix" | "imat" => {
            let n = size(s.group()?)?;
            grid(n, n, |i, j| if i == j { "1" } else { "0" }.to_string())
        }
        "zeromatrix" | "zmat" => {
            let (m, n) = (size(s.group()?)?, size(s.group()?)?);
            grid(m, n, |_, _| "0".to_string())
        }
        "diagonalmatrix" | "dmat" | "antidiagonalmatrix" | "admat" => {
            let fill = s.optional().map(expand).unwrap_or_default();
            let items: Vec<String> = s.group()?.split(',').map(|i| expand(i.trim())).collect();
            let n = items.len();
            let anti = name.starts_with("anti") || name == "admat";
            grid(n, n, |i, j| if j == if anti { n - 1 - i } else { i } { items[i].clone() } else { fill.clone() })
        }
        _ => return None,
    };
    Some(text)
}

/// `inner` between the delimiters `open` stands for, sized to it unless
/// `star`.
fn fence(open: u8, inner: &str, star: bool) -> String {
    let (l, r) = match open {
        b'(' => ("(", ")"),
        b'[' => ("[", "]"),
        b'{' => ("\\{", "\\}"),
        _ => ("|", "|"),
    };
    sized(l, &[inner.to_string()], r, star)
}

/// `parts` between `l` and `r`, split by bars, sized to them unless `star`.
fn sized(l: &str, parts: &[String], r: &str, star: bool) -> String {
    if star {
        format!("{l} {} {r}", parts.join(" | "))
    } else {
        format!("\\left{l} {} \\right{r}", parts.join(" \\middle| "))
    }
}

fn bold(x: &str, star: bool) -> String {
    if star { format!("\\boldsymbol{{{x}}}") } else { format!("\\mathbf{{{x}}}") }
}

fn power(n: Option<String>) -> String {
    n.map(|n| format!("^{{{n}}}")).unwrap_or_default()
}

/// What follows an operator: `(…)` and `[…]` sized to it, `{…}` as a group.
fn operand(s: &mut Scan) -> String {
    match s.delimited(b"([{") {
        Some((b'{', body)) => format!("{{{}}}", expand(body)),
        Some((open, body)) => fence(open, &expand(body), false),
        None => String::new(),
    }
}

/// `\dv`, `\pdv` and `\fdv`, with `d` for their d: `[n]` an order, then
/// the function and up to `most` variables, or only the variable (then
/// optionally what it acts on in parentheses). Starred, inline as a slash.
fn derivative(d: &str, most: usize, s: &mut Scan) -> Option<String> {
    let star = s.star();
    let n = s.optional().map(expand);
    let first = expand(s.group()?);
    let mut vars = Vec::new();
    while vars.len() < most
        && let Some(var) = s.group()
    {
        vars.push(expand(var));
    }
    let up = power(n);
    let (top, bottom, after) = match vars.as_slice() {
        [] => {
            let after = match s.delimited(b"(") {
                Some((_, body)) => fence(b'(', &expand(body), false),
                None => String::new(),
            };
            (format!("{d}{up}"), format!("{d} {first}{up}"), after)
        }
        [x] => (format!("{d}{up} {first}"), format!("{d} {x}{up}"), String::new()),
        [x, y, ..] => (format!("{d}^{{2}} {first}"), format!("{d} {x} {d} {y}"), String::new()),
    };
    Some(if star { format!("{top} / {bottom}{after}") } else { format!("\\frac{{{top}}}{{{bottom}}}{after}") })
}

fn matrix(env: &str, body: &str) -> String {
    format!("\\begin{{{env}}} {body} \\end{{{env}}}")
}

/// A matrix's size: a small whole number.
fn size(s: &str) -> Option<usize> {
    s.trim().parse().ok().filter(|n| (1..=20).contains(n))
}

/// The rows of an `m` by `n` matrix, `cell` saying what each holds.
fn grid(m: usize, n: usize, cell: impl Fn(usize, usize) -> String) -> String {
    (0..m).map(|i| (0..n).map(|j| cell(i, j)).collect::<Vec<_>>().join(" & ")).collect::<Vec<_>>().join(" \\\\ ")
}

/// Reading a command's arguments from `tex` at `at`.
struct Scan<'a> {
    tex: &'a str,
    at: usize,
}

impl<'a> Scan<'a> {
    fn byte(&self) -> Option<u8> {
        self.tex.as_bytes().get(self.at).copied()
    }

    /// The name of the command whose `\` is here, empty for a control
    /// symbol, moving past it.
    fn word(&mut self) -> &'a str {
        let rest = &self.tex[self.at + 1..];
        let n = rest.bytes().take_while(u8::is_ascii_alphabetic).count();
        self.at += 1 + n;
        &rest[..n]
    }

    fn star(&mut self) -> bool {
        let star = self.byte() == Some(b'*');
        self.at += usize::from(star);
        star
    }

    /// What is between the delimiter here, after any spaces, if it is one
    /// of `opens`, and the one that matches it, and which it was. Nothing
    /// is read when there is none.
    fn delimited(&mut self, opens: &[u8]) -> Option<(u8, &'a str)> {
        let bytes = self.tex.as_bytes();
        let mut start = self.at;
        while bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
            start += 1;
        }
        let open = *bytes.get(start).filter(|b| opens.contains(b))?;
        let close = match open {
            b'(' => b')',
            b'[' => b']',
            b'{' => b'}',
            _ => b'|',
        };
        let (mut depth, mut braces) = (0, 0);
        let mut i = start + 1;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' => i += 1,
                b if b == close && depth == 0 && braces == 0 => {
                    self.at = i + 1;
                    return Some((open, &self.tex[start + 1..i]));
                }
                b'{' => braces += 1,
                b'}' => braces -= 1,
                b if b == open && open != b'|' => depth += 1,
                b if b == close => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        None
    }

    fn group(&mut self) -> Option<&'a str> {
        self.delimited(b"{").map(|(_, body)| body)
    }

    fn optional(&mut self) -> Option<&'a str> {
        self.delimited(b"[").map(|(_, body)| body)
    }

    /// An argument, written out: a group, or the one token here.
    fn arg(&mut self) -> Option<String> {
        if let Some(body) = self.group() {
            return Some(expand(body));
        }
        while self.byte().is_some_and(|b| b.is_ascii_whitespace()) {
            self.at += 1;
        }
        let start = self.at;
        match self.byte()? {
            b'}' | b'&' | b'^' | b'_' => return None,
            b'\\' if self.word().is_empty() => self.at = (self.at + 1).min(self.tex.len()),
            b'\\' => {}
            _ => self.at += self.tex[start..].chars().next()?.len_utf8(),
        }
        Some(expand(&self.tex[start..self.at]))
    }

    /// What a sized pair holds: `(…)`, `[…]`, `|…|` or a group's insides.
    fn inner(&mut self) -> Option<String> {
        match self.delimited(b"([{|") {
            Some((_, body)) => Some(expand(body)),
            None => self.arg(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_out_the_package() {
        assert_eq!(expand(r"\qty(\frac{a}{b})"), r"\left( \frac{a}{b} \right)");
        assert_eq!(expand(r"\qty*[x]"), r"[ x ]");
        assert_eq!(expand(r"\abs{x} + \norm{\vb{v}}"), r"\left| x \right| + \left\| \mathbf{v} \right\|");
        assert_eq!(expand(r"\eval{x^2}_0^1"), r"\left. x^2 \right|_0^1");
        assert_eq!(expand(r"\comm{A}{B}"), r"\left[ A , B \right]");
        assert_eq!(expand(r"\grad\Psi = \div(\vb*{E})"), r"\nabla \Psi = \nabla\cdot \left( \boldsymbol{E} \right)");
        assert_eq!(expand(r"\vu{x} \cross \va{y}"), r"\hat{\mathbf{x}} \times  \vec{\mathbf{y}}");
        assert_eq!(expand(r"\Tr\rho"), r"\operatorname{Tr}\rho");
        assert_eq!(expand(r"\Re(z)"), r"\operatorname{Re}\left( z \right)");
        assert_eq!(expand(r"x \qq{for all} y \qand z"), r"x \quad\text{for all}\quad  y \quad\text{and}\quad  z");
        assert_eq!(expand(r"\dd{x}"), r"\mathrm{d} x");
        assert_eq!(expand(r"\dd[3]{x}"), r"\mathrm{d}^{3} x");
        assert_eq!(expand(r"\dv{f}{x}"), r"\frac{\mathrm{d} f}{\mathrm{d} x}");
        assert_eq!(expand(r"\dv[2]{f}{x}"), r"\frac{\mathrm{d}^{2} f}{\mathrm{d} x^{2}}");
        assert_eq!(expand(r"\dv{x}(x^2)"), r"\frac{\mathrm{d}}{\mathrm{d} x}\left( x^2 \right)");
        assert_eq!(expand(r"\pdv{f}{x}{y}"), r"\frac{\partial^{2} f}{\partial x \partial y}");
        assert_eq!(expand(r"\pdv*{f}{x}"), r"\partial f / \partial x");
        assert_eq!(expand(r"\braket{a}{b}"), r"\left\langle a \middle| b \right\rangle");
        assert_eq!(expand(r"\ket{\psi}"), r"\left| \psi \right\rangle");
        assert_eq!(expand(r"\expval{A}{\psi}"), r"\left\langle \psi \middle| A \middle| \psi \right\rangle");
        assert_eq!(expand(r"\mel{n}{H}{m}"), r"\left\langle n \middle| H \middle| m \right\rangle");
        assert_eq!(expand(r"\mqty[a & b \\ c & d]"), r"\begin{bmatrix} a & b \\ c & d \end{bmatrix}");
        assert_eq!(expand(r"\mqty(\imat{2})"), r"\begin{pmatrix} 1 & 0 \\ 0 & 1 \end{pmatrix}");
        assert_eq!(expand(r"\dmat[0]{1,2}"), r"1 & 0 \\ 0 & 2");
        assert_eq!(expand(r"\admat{a,b}"), r" & a \\ b & ");
    }

    #[test]
    fn typesets_what_it_writes() {
        for tex in [
            r"\qty(\frac{a}{b}) \pqty{x} \bqty{x} \vqty{x} \Bqty{x} \abs{x} \norm*{x} \eval{x}_0^1 \order{x^2}",
            r"\comm{A}{B} \acomm{A}{B} \pb{f}{g} \vb{a} \va{a} \vu*{a} \vdot \cross \grad f \div{E} \curl(B) \laplacian\phi",
            r"\tr\rho \Re(z) \Im z \pv\int f \PV x \qq{text} \qc \qcc \qif x \qotherwise",
            r"\dd{x} \dd[2](x) \var{f} \dv{f}{x} \dv*[2]{f}{x} \dv{x}(f) \pdv{f}{x}{y} \fdv{F}{\phi}",
            r"\bra{a} \ket*{b} \braket{a}{b} \braket{a} \ketbra{a}{b} \expval{A} \expval{A}{\psi} \mel{n}{H}{m}",
            r"\mqty(a & b \\ c & d) \mqty|a| \pmqty{\imat{2}} \bmqty{\zmat{2}{3}} \vmqty{\dmat{1,2}} \mqty(\admat[0]{1,2,3})",
        ] {
            assert_eq!(crate::math::check(&expand(tex), true), Ok(()), "{tex}");
        }
    }

    #[test]
    fn leaves_the_rest_alone() {
        for tex in [r"a \\ b \{ c \} \frac{x}{y} \quad", r"\qty", r"\dv x", "é \\alpha", r"\left| x \right|"] {
            assert_eq!(expand(tex), tex);
        }
        // Writing out again changes nothing.
        let once = expand(r"\pdv[2]{\abs{\psi}}{x} \qif \bra{a}\ket{b} \mqty|\zmat{1}{2}|");
        assert_eq!(expand(&once), once);
    }
}
