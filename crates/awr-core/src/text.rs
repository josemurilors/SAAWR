//! Byte-level line and token helpers for the game's data files.

/// Lines without their `\r\n` / `\n` terminator.
pub fn lines(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    bytes
        .split(|&b| b == b'\n')
        .map(|l| l.strip_suffix(b"\r").unwrap_or(l))
}

/// Accented Latin letters folded to plain ASCII (`Exército` -> `Exercito`); other
/// non-ASCII characters are kept so the caller can decide what to do with them.
pub fn fold_ascii(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'A',
            'ç' => 'c',
            'Ç' => 'C',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'È' | 'É' | 'Ê' | 'Ë' => 'E',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'Ì' | 'Í' | 'Î' | 'Ï' => 'I',
            'ñ' => 'n',
            'Ñ' => 'N',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => 'O',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'Ù' | 'Ú' | 'Û' | 'Ü' => 'U',
            'ý' | 'ÿ' => 'y',
            'Ý' => 'Y',
            c => c,
        })
        .collect()
}

/// Token byte spans, splitting on whitespace and commas like the game's own
/// parser does (the original vehicles.ide has lines with missing commas).
pub fn tokens(line: &[u8]) -> Vec<(usize, usize)> {
    let sep = |b: u8| b.is_ascii_whitespace() || b == b',';
    let mut out = vec![];
    let mut start = None;
    for (i, &b) in line.iter().enumerate() {
        match (start, sep(b)) {
            (None, false) => start = Some(i),
            (Some(s), true) => {
                out.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, line.len()));
    }
    out
}

/// Copy of `line` with the given token spans replaced, separators untouched.
pub fn replace_spans(line: &[u8], edits: &[((usize, usize), &[u8])]) -> Vec<u8> {
    let mut edits = edits.to_vec();
    edits.sort_by_key(|(span, _)| span.0);
    let mut out = Vec::with_capacity(line.len() + 8);
    let mut pos = 0;
    for ((s, e), new) in edits {
        out.extend_from_slice(&line[pos..s]);
        out.extend_from_slice(new);
        pos = e;
    }
    out.extend_from_slice(&line[pos..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_malformed_ide_line_like_the_game() {
        let l = b"585,\temperor\t\temperor, \tcar";
        let t: Vec<&[u8]> = tokens(l).iter().map(|&(s, e)| &l[s..e]).collect();
        assert_eq!(t, [&b"585"[..], b"emperor", b"emperor", b"car"]);
    }

    #[test]
    fn replace_keeps_separators() {
        let l = b"LANDSTAL     1700.0\t 85";
        let t = tokens(l);
        let out = replace_spans(l, &[(t[0], b"GTR34"), (t[2], b"99")]);
        assert_eq!(out, b"GTR34     1700.0\t 99");
    }

    #[test]
    fn folds_accents() {
        assert_eq!(
            fold_ascii("Exército Brasileiro, Ação Ñandú"),
            "Exercito Brasileiro, Acao Nandu"
        );
    }

    #[test]
    fn lines_strip_crlf() {
        let v: Vec<_> = lines(b"a\r\nb\nc").collect();
        assert_eq!(v, [&b"a"[..], b"b", b"c"]);
    }
}
