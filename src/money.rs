use anyhow::{Result, bail};

/// Parses `12,50`, `12.50`, `12` or `0` into cents without floating point.
pub fn parse_cents(input: &str) -> Result<i64> {
    let input = input.trim();
    let (whole, frac) = match input.split_once([',', '.']) {
        Some((whole, frac)) => (whole, frac),
        None => (input, ""),
    };
    let has_separator = whole.len() != input.len();
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || (has_separator && !digits(frac)) {
        bail!("valor inválido: {input:?}");
    }
    if frac.len() > 2 {
        bail!("valor com mais de 2 casas decimais: {input:?}");
    }
    let cents = format!("{frac:0<2}").parse::<i64>()?;
    whole
        .parse::<i64>()?
        .checked_mul(100)
        .and_then(|w| w.checked_add(cents))
        .ok_or_else(|| anyhow::anyhow!("valor demasiado grande: {input:?}"))
}

/// Cents as `12,50`, the form the edit field accepts back.
pub fn format_decimal(cents: i64) -> String {
    let abs = cents.unsigned_abs();
    format!("{},{:02}", abs / 100, abs % 100)
}

pub fn format_cents(cents: i64) -> String {
    format!("{} €", format_decimal(cents))
}

/// Signed format for the difference: `+288,90 €` or `−12,00 €`.
pub fn format_signed(cents: i64) -> String {
    let sign = if cents < 0 { '−' } else { '+' };
    format!("{sign}{}", format_cents(cents))
}
