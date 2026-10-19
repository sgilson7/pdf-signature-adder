//! PDF objects: the eight kinds a file is made of, read from bytes and
//! written back. Names are kept decoded (`#20` becomes a space) so keys
//! compare as written in the specification, and are encoded again on output.
//! Strings are written back as hexadecimal, which needs no escaping.

/// One PDF object. Reals keep their text, so a value copied from the original
/// file is written back exactly as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Obj {
    Null,
    Bool(bool),
    Int(i64),
    Real(String),
    Name(Vec<u8>),
    Str(Vec<u8>),
    Array(Vec<Obj>),
    Dict(Dict),
    Ref(u32, u16),
}

/// A dictionary, in the order the file gave it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Dict(pub Vec<(Vec<u8>, Obj)>);

impl Dict {
    pub fn get(&self, key: &str) -> Option<&Obj> {
        self.0.iter().find(|(k, _)| k == key.as_bytes()).map(|(_, v)| v)
    }

    /// Replaces the value under `key`, or appends it.
    pub fn set(&mut self, key: &str, value: Obj) {
        match self.0.iter_mut().find(|(k, _)| k == key.as_bytes()) {
            Some(slot) => slot.1 = value,
            None => self.0.push((key.as_bytes().to_vec(), value)),
        }
    }

    pub fn remove(&mut self, key: &str) {
        self.0.retain(|(k, _)| k != key.as_bytes());
    }

    pub fn name(&self, key: &str) -> Option<&[u8]> {
        match self.get(key) {
            Some(Obj::Name(n)) => Some(n),
            _ => None,
        }
    }

    pub fn int(&self, key: &str) -> Option<i64> {
        match self.get(key) {
            Some(Obj::Int(n)) => Some(*n),
            _ => None,
        }
    }
}

impl Obj {
    pub fn as_dict(&self) -> Option<&Dict> {
        match self {
            Obj::Dict(d) => Some(d),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Obj]> {
        match self {
            Obj::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Obj::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// The value in thousandths, for an integer or a real.
    pub fn as_milli(&self) -> Option<i64> {
        match self {
            Obj::Int(n) => n.checked_mul(crate::num::MILLI),
            Obj::Real(text) => crate::num::parse_milli(text),
            _ => None,
        }
    }

    pub fn name(text: &str) -> Obj {
        Obj::Name(text.as_bytes().to_vec())
    }
}

/// Why bytes could not be read as an object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub at: usize,
    pub what: &'static str,
}

pub fn is_white(b: u8) -> bool {
    matches!(b, 0 | 9 | 10 | 12 | 13 | 32)
}

pub fn is_delim(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

fn is_regular(b: u8) -> bool {
    !is_white(b) && !is_delim(b)
}

/// Nesting deeper than this is refused rather than followed: real files nest
/// a handful of levels, and a crafted file could nest until the stack runs out.
const MAX_DEPTH: usize = 64;

/// Reads objects from a byte slice, from a position.
pub struct Lexer<'a> {
    pub bytes: &'a [u8],
    pub pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(bytes: &'a [u8], pos: usize) -> Lexer<'a> {
        Lexer { bytes, pos }
    }

    fn err<T>(&self, what: &'static str) -> Result<T, SyntaxError> {
        Err(SyntaxError { at: self.pos, what })
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    /// Skips white space and comments.
    pub fn skip_white(&mut self) {
        while let Some(b) = self.peek() {
            if is_white(b) {
                self.pos += 1;
            } else if b == b'%' {
                while let Some(c) = self.peek() {
                    if c == b'\n' || c == b'\r' {
                        break;
                    }
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    /// The next run of regular characters, without consuming white space
    /// after it.
    pub fn word(&mut self) -> &'a [u8] {
        self.skip_white();
        let start = self.pos;
        while self.peek().map(is_regular).unwrap_or(false) {
            self.pos += 1;
        }
        &self.bytes[start..self.pos]
    }

    /// Consumes `keyword` if it comes next, as a whole word.
    pub fn keyword(&mut self, keyword: &str) -> bool {
        let save = self.pos;
        if self.word() == keyword.as_bytes() {
            true
        } else {
            self.pos = save;
            false
        }
    }

    /// A non-negative integer word, if one comes next.
    pub fn uint(&mut self) -> Option<u64> {
        let save = self.pos;
        let w = self.word();
        if w.is_empty() || w.len() > 18 || !w.iter().all(|b| b.is_ascii_digit()) {
            self.pos = save;
            return None;
        }
        Some(w.iter().fold(0u64, |n, b| n * 10 + u64::from(b - b'0')))
    }

    pub fn object(&mut self) -> Result<Obj, SyntaxError> {
        self.object_at_depth(0)
    }

    fn object_at_depth(&mut self, depth: usize) -> Result<Obj, SyntaxError> {
        if depth > MAX_DEPTH {
            return self.err("objects nested too deeply");
        }
        self.skip_white();
        let Some(b) = self.peek() else {
            return self.err("the file ends inside an object");
        };
        match b {
            b'/' => {
                self.pos += 1;
                Ok(Obj::Name(self.name_body()))
            }
            b'(' => {
                self.pos += 1;
                self.literal_string().map(Obj::Str)
            }
            b'<' if self.bytes.get(self.pos + 1) == Some(&b'<') => {
                self.pos += 2;
                let mut dict = Dict::default();
                loop {
                    self.skip_white();
                    match self.peek() {
                        Some(b'>') if self.bytes.get(self.pos + 1) == Some(&b'>') => {
                            self.pos += 2;
                            return Ok(Obj::Dict(dict));
                        }
                        Some(b'/') => {
                            self.pos += 1;
                            let key = self.name_body();
                            let value = self.object_at_depth(depth + 1)?;
                            // A later duplicate key wins, as in most readers.
                            dict.0.retain(|(k, _)| *k != key);
                            dict.0.push((key, value));
                        }
                        _ => return self.err("a dictionary key is not a name"),
                    }
                }
            }
            b'<' => {
                self.pos += 1;
                self.hex_string().map(Obj::Str)
            }
            b'[' => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_white();
                    match self.peek() {
                        Some(b']') => {
                            self.pos += 1;
                            return Ok(Obj::Array(items));
                        }
                        None => return self.err("the file ends inside an array"),
                        _ => items.push(self.object_at_depth(depth + 1)?),
                    }
                }
            }
            _ => self.number_or_keyword(),
        }
    }

    fn name_body(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(b) = self.peek() {
            if !is_regular(b) {
                break;
            }
            if b == b'#' {
                let hi = self.bytes.get(self.pos + 1).and_then(|c| hex_value(*c));
                let lo = self.bytes.get(self.pos + 2).and_then(|c| hex_value(*c));
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push(h * 16 + l);
                    self.pos += 3;
                    continue;
                }
            }
            out.push(b);
            self.pos += 1;
        }
        out
    }

    fn literal_string(&mut self) -> Result<Vec<u8>, SyntaxError> {
        let mut out = Vec::new();
        let mut depth = 1;
        loop {
            let Some(b) = self.peek() else {
                return self.err("the file ends inside a string");
            };
            self.pos += 1;
            match b {
                b'(' => {
                    depth += 1;
                    out.push(b);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(out);
                    }
                    out.push(b);
                }
                b'\\' => {
                    let Some(e) = self.peek() else {
                        return self.err("the file ends inside a string");
                    };
                    self.pos += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'\r' => {
                            if self.peek() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        b'\n' => {}
                        b'0'..=b'7' => {
                            let mut v = u32::from(e - b'0');
                            for _ in 0..2 {
                                match self.peek() {
                                    Some(d @ b'0'..=b'7') => {
                                        v = v * 8 + u32::from(d - b'0');
                                        self.pos += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push((v & 0xff) as u8);
                        }
                        other => out.push(other),
                    }
                }
                _ => out.push(b),
            }
        }
    }

    fn hex_string(&mut self) -> Result<Vec<u8>, SyntaxError> {
        let mut nibbles = Vec::new();
        loop {
            let Some(b) = self.peek() else {
                return self.err("the file ends inside a string");
            };
            self.pos += 1;
            if b == b'>' {
                break;
            }
            if is_white(b) {
                continue;
            }
            match hex_value(b) {
                Some(v) => nibbles.push(v),
                None => return self.err("a hexadecimal string holds a non-hex character"),
            }
        }
        if nibbles.len() % 2 == 1 {
            nibbles.push(0);
        }
        Ok(nibbles.chunks(2).map(|p| p[0] * 16 + p[1]).collect())
    }

    fn number_or_keyword(&mut self) -> Result<Obj, SyntaxError> {
        let start = self.pos;
        let w = self.word();
        if w.is_empty() {
            self.pos = start + 1;
            return Err(SyntaxError {
                at: start,
                what: "an unexpected character",
            });
        }
        match w {
            b"true" => return Ok(Obj::Bool(true)),
            b"false" => return Ok(Obj::Bool(false)),
            b"null" => return Ok(Obj::Null),
            _ => {}
        }
        let text = std::str::from_utf8(w).unwrap_or("");
        let numeric = !text.is_empty()
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || b == b'.' || b == b'-' || b == b'+');
        if !numeric {
            self.pos = start;
            return self.err("an unknown keyword");
        }
        if text.contains('.') {
            return Ok(Obj::Real(text.to_string()));
        }
        // Some writers put "--5" or "5-"; read what parses, refuse the rest.
        let value: i64 = match text.parse() {
            Ok(v) => v,
            Err(_) => return Ok(Obj::Real(text.to_string())),
        };
        // "12 0 R" is a reference: two non-negative integers and R.
        if value >= 0 && value <= i64::from(u32::MAX) {
            let save = self.pos;
            if let Some(generation) = self.uint() {
                if generation <= u64::from(u16::MAX) && self.keyword("R") {
                    return Ok(Obj::Ref(value as u32, generation as u16));
                }
            }
            self.pos = save;
        }
        Ok(Obj::Int(value))
    }
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

const HEX: &[u8; 16] = b"0123456789ABCDEF";

/// Writes an object as PDF syntax.
pub fn write(obj: &Obj, out: &mut Vec<u8>) {
    match obj {
        Obj::Null => out.extend_from_slice(b"null"),
        Obj::Bool(true) => out.extend_from_slice(b"true"),
        Obj::Bool(false) => out.extend_from_slice(b"false"),
        Obj::Int(n) => out.extend_from_slice(n.to_string().as_bytes()),
        Obj::Real(text) => out.extend_from_slice(text.as_bytes()),
        Obj::Name(n) => write_name(n, out),
        Obj::Str(s) => {
            out.push(b'<');
            for b in s {
                out.push(HEX[usize::from(b >> 4)]);
                out.push(HEX[usize::from(b & 15)]);
            }
            out.push(b'>');
        }
        Obj::Array(items) => {
            out.push(b'[');
            for (k, item) in items.iter().enumerate() {
                if k > 0 {
                    out.push(b' ');
                }
                write(item, out);
            }
            out.push(b']');
        }
        Obj::Dict(d) => write_dict(d, out),
        Obj::Ref(n, g) => out.extend_from_slice(format!("{n} {g} R").as_bytes()),
    }
}

pub fn write_dict(d: &Dict, out: &mut Vec<u8>) {
    out.extend_from_slice(b"<<");
    for (k, v) in &d.0 {
        write_name(k, out);
        out.push(b' ');
        write(v, out);
    }
    out.extend_from_slice(b">>");
}

fn write_name(n: &[u8], out: &mut Vec<u8>) {
    out.push(b'/');
    for &b in n {
        if b > 32 && b < 127 && b != b'#' && !is_delim(b) {
            out.push(b);
        } else {
            out.push(b'#');
            out.push(HEX[usize::from(b >> 4)]);
            out.push(HEX[usize::from(b & 15)]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Obj {
        Lexer::new(text.as_bytes(), 0).object().unwrap()
    }

    fn written(obj: &Obj) -> String {
        let mut out = Vec::new();
        write(obj, &mut out);
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn each_kind_of_object_reads() {
        assert_eq!(read("null"), Obj::Null);
        assert_eq!(read(" true"), Obj::Bool(true));
        assert_eq!(read("-42"), Obj::Int(-42));
        assert_eq!(read("0.88"), Obj::Real("0.88".into()));
        assert_eq!(read("/Media#20Box"), Obj::Name(b"Media Box".to_vec()));
        assert_eq!(read("(a\\(b\\)\\101\\\nc)"), Obj::Str(b"a(b)Ac".to_vec()));
        assert_eq!(read("(x(y)z)"), Obj::Str(b"x(y)z".to_vec()));
        assert_eq!(read("<48 69 9>"), Obj::Str(b"Hi\x90".to_vec()));
        assert_eq!(read("12 0 R"), Obj::Ref(12, 0));
        assert_eq!(
            read("[1 2 0 R /A]"),
            Obj::Array(vec![Obj::Int(1), Obj::Ref(2, 0), Obj::name("A")])
        );
    }

    #[test]
    fn two_integers_without_r_stay_integers() {
        assert_eq!(
            read("[0 0 612 792]"),
            Obj::Array(vec![Obj::Int(0), Obj::Int(0), Obj::Int(612), Obj::Int(792)])
        );
    }

    #[test]
    fn a_dictionary_keeps_its_order_and_a_comment_is_skipped() {
        let d = read("<</Type/Page % a comment\n/Parent 3 0 R/Rotate 90>>");
        let d = d.as_dict().unwrap();
        assert_eq!(d.name("Type"), Some(&b"Page"[..]));
        assert_eq!(d.get("Parent"), Some(&Obj::Ref(3, 0)));
        assert_eq!(d.int("Rotate"), Some(90));
        assert_eq!(d.0[0].0, b"Type".to_vec());
    }

    #[test]
    fn objects_write_back_as_pdf_syntax() {
        let d = read("<</A [1 0.5 (x)] /B#20C 4 0 R /D true>>");
        assert_eq!(written(&d), "<</A [1 0.5 <78>]/B#20C 4 0 R/D true>>");
    }

    #[test]
    fn deep_nesting_is_refused() {
        let text = "[".repeat(100);
        let e = Lexer::new(text.as_bytes(), 0).object().unwrap_err();
        assert_eq!(e.what, "objects nested too deeply");
    }

    #[test]
    fn an_unterminated_string_is_refused() {
        let e = Lexer::new(b"(abc", 0).object().unwrap_err();
        assert_eq!(e.what, "the file ends inside a string");
    }
}
