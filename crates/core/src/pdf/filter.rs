//! Stream filters the reader needs: Flate, with the PNG predictors that
//! cross-reference streams use. Content streams are never decoded; the tool
//! adds its own content beside them.

use super::object::{Dict, Obj};

/// The largest decoded stream the reader accepts. Cross-reference and object
/// streams are far smaller; this stops a crafted stream from inflating
/// without end.
const MAX_DECODED: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterError {
    Unsupported(String),
    Corrupt,
}

/// Decodes a stream's data by its `/Filter` and `/DecodeParms`.
pub fn decode(dict: &Dict, data: &[u8]) -> Result<Vec<u8>, FilterError> {
    let filters: Vec<Vec<u8>> = match dict.get("Filter") {
        None => Vec::new(),
        Some(Obj::Name(n)) => vec![n.clone()],
        Some(Obj::Array(items)) => items
            .iter()
            .map(|i| match i {
                Obj::Name(n) => Ok(n.clone()),
                _ => Err(FilterError::Corrupt),
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(FilterError::Corrupt),
    };
    let params: Vec<Option<&Dict>> = match dict.get("DecodeParms") {
        Some(Obj::Dict(d)) => vec![Some(d)],
        Some(Obj::Array(items)) => items.iter().map(|i| i.as_dict()).collect(),
        _ => Vec::new(),
    };
    let mut bytes = data.to_vec();
    for (k, f) in filters.iter().enumerate() {
        bytes = match f.as_slice() {
            b"FlateDecode" | b"Fl" => {
                let raw = inflate(&bytes)?;
                predict(&raw, params.get(k).copied().flatten())?
            }
            other => {
                return Err(FilterError::Unsupported(
                    String::from_utf8_lossy(other).into_owned(),
                ))
            }
        };
    }
    Ok(bytes)
}

/// Zlib inflate; falls back to raw deflate after the two header bytes, since
/// some writers leave out or spoil the checksum.
pub fn inflate(data: &[u8]) -> Result<Vec<u8>, FilterError> {
    if let Ok(v) = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(data, MAX_DECODED) {
        return Ok(v);
    }
    if data.len() > 2 {
        if let Ok(v) = miniz_oxide::inflate::decompress_to_vec_with_limit(&data[2..], MAX_DECODED) {
            return Ok(v);
        }
    }
    Err(FilterError::Corrupt)
}

/// Zlib deflate at the default level, for what the tool writes.
pub fn deflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(data, 6)
}

/// Undoes a PNG predictor (10 to 15). Predictor 1 or none passes through.
fn predict(data: &[u8], params: Option<&Dict>) -> Result<Vec<u8>, FilterError> {
    let Some(p) = params else {
        return Ok(data.to_vec());
    };
    let predictor = p.int("Predictor").unwrap_or(1);
    if predictor <= 1 {
        return Ok(data.to_vec());
    }
    if predictor < 10 {
        return Err(FilterError::Unsupported(format!("Predictor {predictor}")));
    }
    let colors = p.int("Colors").unwrap_or(1);
    let bits = p.int("BitsPerComponent").unwrap_or(8);
    let columns = p.int("Columns").unwrap_or(1);
    if !(1..=32).contains(&colors) || ![1, 2, 4, 8, 16].contains(&bits) || !(1..=1_000_000).contains(&columns)
    {
        return Err(FilterError::Corrupt);
    }
    let bpp = ((colors * bits + 7) / 8) as usize;
    let row = ((colors * bits * columns + 7) / 8) as usize;
    let mut out = Vec::with_capacity(data.len());
    let mut prev = vec![0u8; row];
    for chunk in data.chunks(row + 1) {
        if chunk.len() < row + 1 {
            break;
        }
        let kind = chunk[0];
        let mut cur = chunk[1..].to_vec();
        for i in 0..row {
            let left = if i >= bpp { cur[i - bpp] } else { 0 };
            let up = prev[i];
            let up_left = if i >= bpp { prev[i - bpp] } else { 0 };
            let add = match kind {
                0 => 0,
                1 => left,
                2 => up,
                3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                4 => paeth(left, up, up_left),
                _ => return Err(FilterError::Corrupt),
            };
            cur[i] = cur[i].wrapping_add(add);
        }
        out.extend_from_slice(&cur);
        prev = cur;
    }
    Ok(out)
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i16::from(a) + i16::from(b) - i16::from(c);
    let pa = (p - i16::from(a)).abs();
    let pb = (p - i16::from(b)).abs();
    let pc = (p - i16::from(c)).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::object::Lexer;

    fn dict(text: &str) -> Dict {
        Lexer::new(text.as_bytes(), 0)
            .object()
            .unwrap()
            .as_dict()
            .unwrap()
            .clone()
    }

    #[test]
    fn flate_round_trips() {
        let data = b"q 1 0 0 1 0 0 cm Q".repeat(20);
        let packed = deflate(&data);
        assert!(packed.len() < data.len());
        assert_eq!(decode(&dict("<</Filter/FlateDecode>>"), &packed).unwrap(), data);
    }

    #[test]
    fn png_up_predictor_adds_the_row_above() {
        // Two rows of three columns, each row tagged 2 (Up), worked by hand:
        // row 1 = 1 2 3; row 2 = 1+1 2+1 3+1.
        let rows = [2u8, 1, 2, 3, 2, 1, 1, 1];
        let packed = deflate(&rows);
        let d = dict("<</Filter/FlateDecode/DecodeParms<</Predictor 12/Columns 3>>>>");
        assert_eq!(decode(&d, &packed).unwrap(), vec![1, 2, 3, 2, 3, 4]);
    }

    #[test]
    fn png_sub_and_paeth_predictors() {
        // Sub: 5, 5+1, 6+1. Paeth on row 2 (above 5 6 7): first byte left=0,
        // up=5, upleft=0 -> 5, so 1+5=6; then left=6, up=6, upleft=5: p=7,
        // pa=1, pb=1, pc=2 -> a (6): 1+6=7; then left=7,up=7,upleft=6 -> 7: 0+7.
        let rows = [1u8, 5, 1, 1, 4, 1, 1, 0];
        let d = dict("<</Filter/FlateDecode/DecodeParms<</Predictor 12/Columns 3>>>>");
        assert_eq!(decode(&d, &deflate(&rows)).unwrap(), vec![5, 6, 7, 6, 7, 7]);
    }

    #[test]
    fn an_unknown_filter_is_named() {
        assert_eq!(
            decode(&dict("<</Filter/LZWDecode>>"), b"x"),
            Err(FilterError::Unsupported("LZWDecode".into()))
        );
    }

    #[test]
    fn spoiled_data_is_corrupt() {
        assert_eq!(
            decode(&dict("<</Filter/FlateDecode>>"), b"not zlib at all"),
            Err(FilterError::Corrupt)
        );
    }
}
