// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub fn difficulty_from_bits(bits_str: &str) -> String {
    let bits = match u32::from_str_radix(bits_str.trim_start_matches("0x"), 16) {
        Ok(v) => v,
        Err(_) => return "?".to_string(),
    };

    let exponent = (bits >> 24) as i32;
    let mantissa = (bits & 0x007f_ffff) as f64;

    let shift = 8.0 * (exponent as f64 - 3.0);

    // Guard against absurdly large/small shifts
    if shift >= 1024.0 || mantissa == 0.0 {
        return "0".to_string();
    }

    let target = mantissa * 2f64.powf(shift);
    if target == 0.0 {
        return "0".to_string();
    }

    // diff1_target = 0xffff << 208  (same constant as the JS dashboard)
    // Using f64: precision is fine for a display string.
    let diff1 = 65535.0_f64 * 2f64.powf(208.0);
    let difficulty = diff1 / target;

    format_difficulty(difficulty)
}

fn format_difficulty(value: f64) -> String {
    const SUFFIXES: &[&str] = &["", "K", "M", "G", "T", "P", "E"];

    if value < 10_000.0 {
        return format!("{:.0}", value);
    }

    let mut scaled = value;
    let mut tier = 0usize;

    while scaled >= 1_000.0 && tier < SUFFIXES.len() - 1 {
        scaled /= 1_000.0;
        tier += 1;
    }

    if scaled >= 100.0 {
        format!("{:.0}{}", scaled, SUFFIXES[tier])
    } else if scaled >= 10.0 {
        format!("{:.1}{}", scaled, SUFFIXES[tier])
    } else {
        format!("{:.2}{}", scaled, SUFFIXES[tier])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_1d00ffff_is_1() {
        // Genesis block bits — difficulty 1
        let d = difficulty_from_bits("1d00ffff");
        assert_eq!(d, "1");
    }

    #[test]
    fn bad_bits_returns_question_mark() {
        assert_eq!(difficulty_from_bits("nothex"), "?");
    }

    #[test]
    fn zero_mantissa_returns_zero() {
        // exponent=0x1d, mantissa=0 → target = 0 → "0"
        assert_eq!(difficulty_from_bits("1d000000"), "0");
    }
}
