//! Decimal numbers without floating point. Pointer positions arrive from the
//! page as text ("312.5"), and PDF reals are text in the file ("0.88"); both
//! become thousandths in an `i64`, so the same input gives the same output on
//! every machine (BUILDER-SPEC D2).

/// Thousandths in one unit.
pub const MILLI: i64 = 1000;

/// Parses decimal text into thousandths, rounding half away from zero at the
/// fourth decimal. Accepts an optional sign, digits, and one point, with
/// digits on at least one side ("4.", ".5", "-0.25"). Refuses exponents,
/// spaces inside, and anything else, so a mangled value is never guessed at.
pub fn parse_milli(text: &str) -> Option<i64> {
    let t = text.trim();
    let (negative, body) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let (whole, frac) = match body.split_once('.') {
        Some((w, f)) => (w, f),
        None => (body, ""),
    };
    if whole.is_empty() && frac.is_empty() {
        return None;
    }
    if !whole.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Leading zeros carry no value; more than 15 significant digits would
    // overflow once scaled, and no page position needs them.
    let whole = whole.trim_start_matches('0');
    if whole.len() > 15 {
        return None;
    }
    let mut value: i64 = 0;
    for b in whole.bytes() {
        value = value * 10 + i64::from(b - b'0');
    }
    value *= MILLI;
    let digits: Vec<i64> = frac.bytes().map(|b| i64::from(b - b'0')).collect();
    let mut part = 0;
    for k in 0..3 {
        part = part * 10 + digits.get(k).copied().unwrap_or(0);
    }
    if digits.get(3).copied().unwrap_or(0) >= 5 {
        part += 1;
    }
    value += part;
    Some(if negative { -value } else { value })
}

/// Formats thousandths as the shortest decimal: 1500 -> "1.5", -250 ->
/// "-0.25", 2000 -> "2". This is the form written into PDF content streams.
pub fn format_milli(value: i64) -> String {
    format_scaled(value, MILLI, 3)
}

/// Formats `value / scale` with up to `places` decimals, trailing zeros
/// dropped. `scale` is 10 to the power `places`.
pub fn format_scaled(value: i64, scale: i64, places: usize) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let a = value.unsigned_abs();
    let s = scale.unsigned_abs();
    let whole = a / s;
    let frac = a % s;
    if frac == 0 {
        return format!("{sign}{whole}");
    }
    let digits = format!("{frac:0places$}");
    format!("{sign}{whole}.{}", digits.trim_end_matches('0'))
}

/// `a * b / c`, rounded half away from zero, through `i128` so the product
/// cannot overflow. `c` must not be zero; callers check it.
pub fn mul_div(a: i64, b: i64, c: i64) -> i64 {
    let n = i128::from(a) * i128::from(b);
    let d = i128::from(c);
    let q = n / d;
    let r = n % d;
    let round = if (r * 2).abs() >= d.abs() {
        if (n < 0) != (d < 0) {
            -1
        } else {
            1
        }
    } else {
        0
    };
    (q + round) as i64
}

/// The integer square root, rounded down.
pub fn isqrt(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_text_becomes_thousandths() {
        assert_eq!(parse_milli("312.5"), Some(312_500));
        assert_eq!(parse_milli("-0.25"), Some(-250));
        assert_eq!(parse_milli("4."), Some(4000));
        assert_eq!(parse_milli(".5"), Some(500));
        assert_eq!(parse_milli("+7"), Some(7000));
        assert_eq!(parse_milli("  12 "), Some(12_000));
        assert_eq!(parse_milli("0012.0"), Some(12_000));
    }

    #[test]
    fn the_fourth_decimal_rounds_half_away_from_zero() {
        assert_eq!(parse_milli("1.2345"), Some(1235));
        assert_eq!(parse_milli("1.2344"), Some(1234));
        assert_eq!(parse_milli("-1.2345"), Some(-1235));
        assert_eq!(parse_milli("0.0005"), Some(1));
    }

    #[test]
    fn anything_else_is_refused() {
        for bad in [
            "",
            "-",
            ".",
            "1e3",
            "1.2.3",
            "12px",
            "1 2",
            "NaN",
            "--1",
            "9999999999999999",
        ] {
            assert_eq!(parse_milli(bad), None, "{bad}");
        }
    }

    #[test]
    fn thousandths_format_as_the_shortest_decimal() {
        assert_eq!(format_milli(1500), "1.5");
        assert_eq!(format_milli(-250), "-0.25");
        assert_eq!(format_milli(2000), "2");
        assert_eq!(format_milli(0), "0");
        assert_eq!(format_milli(-5), "-0.005");
        assert_eq!(format_scaled(1_234_500, 1_000_000, 6), "1.2345");
    }

    #[test]
    fn mul_div_rounds_half_away_from_zero() {
        assert_eq!(mul_div(10, 1, 4), 3);
        assert_eq!(mul_div(10, 1, 3), 3);
        assert_eq!(mul_div(-10, 1, 4), -3);
        assert_eq!(mul_div(7, 3, 2), 11);
        assert_eq!(mul_div(i64::MAX / 2, 4, 4), i64::MAX / 2);
    }

    #[test]
    fn isqrt_rounds_down() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(15), 3);
        assert_eq!(isqrt(16), 4);
        assert_eq!(isqrt(1_000_000), 1000);
    }
}
