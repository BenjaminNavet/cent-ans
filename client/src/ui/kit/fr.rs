//! French number, money and plural formatting (game/scripts/ui/money.gd and fr_text.gd).
//!
//! One format for the whole UI: digits grouped by three with a no-break space, real minus sign
//! (U+2212), explicit sign for balances and deltas, `₶` (livre tournois). Same output as
//! `economy_balance::signed_livres` in `core/`. Pure functions, no game rule.

use bevy::color::Color;

/// Livre tournois sign.
pub const SYMBOL: &str = "₶";
/// Typographic minus.
pub const MINUS: char = '\u{2212}';
/// No-break space.
pub const NBSP: char = '\u{a0}';

/// Colour of a loss on parchment (dark red).
pub const LOSS_COLOR: Color = Color::srgb(0.55, 0.12, 0.10);
/// Colour of a gain on parchment (dark green).
pub const GAIN_COLOR: Color = Color::srgb(0.18, 0.38, 0.16);
/// Neutral amount colour (ink).
pub const INK_COLOR: Color = Color::srgb(0.22, 0.14, 0.07);

/// Abbreviated French months of `FrText::MONTHS_SHORT`.
pub const MONTHS_SHORT: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];

/// Place or faction name introduced by « de » with the right elision and article contraction:
/// « France » -> « de France », « Île-de-France » -> « d’Île-de-France », « Le Caire » ->
/// « du Caire », « Les Landes » -> « des Landes », « L’Aigle » -> « de l’Aigle ». An initial h is
/// taken as aspirated (« de Hongrie », « de Hainaut »).
pub fn de_place(name: &str) -> String {
    let name = name.trim();
    if let Some(rest) = name.strip_prefix("Le ") {
        return format!("du {rest}");
    }
    if let Some(rest) = name.strip_prefix("Les ") {
        return format!("des {rest}");
    }
    if let Some(rest) = name.strip_prefix("L’").or_else(|| name.strip_prefix("L'")) {
        return format!("de l’{rest}");
    }
    let vowel = name
        .chars()
        .next()
        .is_some_and(|c| "AEIOUYÀÂÄÉÈÊËÎÏÔÖÛÜŒaeiouyàâäéèêëîïôöûüœ".contains(c));
    if vowel {
        format!("d’{name}")
    } else {
        format!("de {name}")
    }
}

/// `60000` -> « 60 000 » (no symbol; negative with the real minus sign).
pub fn digits(value: i64) -> String {
    let raw = value.unsigned_abs().to_string();
    let mut grouped = String::with_capacity(raw.len() + raw.len() / 3 + 1);
    for (index, digit) in raw.chars().enumerate() {
        if index > 0 && (raw.len() - index).is_multiple_of(3) {
            grouped.push(NBSP);
        }
        grouped.push(digit);
    }
    if value < 0 {
        format!("{MINUS}{grouped}")
    } else {
        grouped
    }
}

/// `60000` -> « 60 000 ₶ ».
pub fn amount(value: i64) -> String {
    format!("{}{NBSP}{SYMBOL}", digits(value))
}

/// `3063` -> « +3 063 ₶ », `-1243` -> « −1 243 ₶ », `0` -> « 0 ₶ ».
pub fn signed(value: i64) -> String {
    if value == 0 {
        return format!("0{NBSP}{SYMBOL}");
    }
    let sign = if value > 0 { '+' } else { MINUS };
    format!("{sign}{}{NBSP}{SYMBOL}", digits(value.abs()))
}

/// A charge (positive value) -> « −9 930 ₶ »; « 0 ₶ » when zero.
pub fn charge(value: i64) -> String {
    if value == 0 {
        format!("0{NBSP}{SYMBOL}")
    } else {
        format!("{MINUS}{}{NBSP}{SYMBOL}", digits(value.abs()))
    }
}

/// Colour of a signed amount (red below zero, green above, ink at zero).
pub fn color_of(value: i64) -> Color {
    match value.cmp(&0) {
        std::cmp::Ordering::Less => LOSS_COLOR,
        std::cmp::Ordering::Greater => GAIN_COLOR,
        std::cmp::Ordering::Equal => INK_COLOR,
    }
}

/// Delta with arrow: « ▲ +1 200 ₶ », « ▼ −900 ₶ », « = » when zero.
pub fn delta(value: i64) -> String {
    match value.cmp(&0) {
        std::cmp::Ordering::Equal => "=".to_owned(),
        std::cmp::Ordering::Greater => format!("▲ {}", signed(value)),
        std::cmp::Ordering::Less => format!("▼ {}", signed(value)),
    }
}

/// « s » when `count` calls for the plural (2 and more; 0 and 1 are singular in French).
pub fn plural_s(count: i64) -> &'static str {
    if count.abs() > 1 { "s" } else { "" }
}

/// « 3 tours », « 1 tour »; `plural` for irregular plurals (« 2 chevaux »).
pub fn count(value: i64, singular: &str, plural: Option<&str>) -> String {
    if value.abs() > 1 {
        format!(
            "{value} {}",
            plural.map_or_else(|| format!("{singular}s"), str::to_owned)
        )
    } else {
        format!("{value} {singular}")
    }
}

/// « 24 sept. 2026, 23 h 19 » (the time part only when `time` is given).
pub fn date(year: i32, month: u32, day: u32, time: Option<(u32, u32)>) -> String {
    let month_name = MONTHS_SHORT[(month.clamp(1, 12) - 1) as usize];
    let mut text = format!("{day} {month_name} {year}");
    if let Some((hour, minute)) = time {
        text.push_str(&format!(", {hour} h {minute:02}"));
    }
    text
}

/// Same format from an ISO string (« 2026-09-24T23:19:05 » or with a space); the input
/// itself when it does not parse.
pub fn date_from_iso(text: &str) -> String {
    let iso = text.trim().replace(' ', "T");
    let parse = || -> Option<String> {
        let (day_part, time_part) = match iso.split_once('T') {
            Some((day_part, time_part)) => (day_part, Some(time_part)),
            None => (iso.as_str(), None),
        };
        let mut fields = day_part.split('-');
        let year: i32 = fields.next()?.parse().ok()?;
        let month: u32 = fields.next()?.parse().ok()?;
        let day: u32 = fields.next()?.parse().ok()?;
        if year == 0 || !(1..=12).contains(&month) {
            return None;
        }
        let time = time_part.and_then(|part| {
            let mut fields = part.split(':');
            Some((fields.next()?.parse().ok()?, fields.next()?.parse().ok()?))
        });
        Some(date(year, month, day, time))
    };
    parse().unwrap_or_else(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_digits_by_three() {
        assert_eq!(digits(0), "0");
        assert_eq!(digits(999), "999");
        assert_eq!(digits(1000), "1\u{a0}000");
        assert_eq!(digits(60000), "60\u{a0}000");
        assert_eq!(digits(1234567), "1\u{a0}234\u{a0}567");
        assert_eq!(digits(-1243), "\u{2212}1\u{a0}243");
    }

    #[test]
    fn money_formats() {
        assert_eq!(amount(60000), "60\u{a0}000\u{a0}₶");
        assert_eq!(signed(3063), "+3\u{a0}063\u{a0}₶");
        assert_eq!(signed(-1243), "\u{2212}1\u{a0}243\u{a0}₶");
        assert_eq!(signed(0), "0\u{a0}₶");
        assert_eq!(charge(9930), "\u{2212}9\u{a0}930\u{a0}₶");
        assert_eq!(charge(0), "0\u{a0}₶");
        assert_eq!(delta(1200), "▲ +1\u{a0}200\u{a0}₶");
        assert_eq!(delta(-900), "▼ \u{2212}900\u{a0}₶");
        assert_eq!(delta(0), "=");
        assert_eq!(color_of(-1), LOSS_COLOR);
        assert_eq!(color_of(5), GAIN_COLOR);
        assert_eq!(color_of(0), INK_COLOR);
    }

    #[test]
    fn plurals_follow_french() {
        assert_eq!(plural_s(0), "");
        assert_eq!(plural_s(1), "");
        assert_eq!(plural_s(2), "s");
        assert_eq!(count(3, "tour", None), "3 tours");
        assert_eq!(count(1, "tour", None), "1 tour");
        assert_eq!(count(0, "tour", None), "0 tour");
        assert_eq!(count(2, "cheval", Some("chevaux")), "2 chevaux");
    }

    #[test]
    fn place_elision() {
        assert_eq!(de_place("Île-de-France"), "d’Île-de-France");
        assert_eq!(de_place("Île de Man"), "d’Île de Man");
        assert_eq!(de_place("Angleterre"), "d’Angleterre");
        assert_eq!(de_place("Évêché de Dorpat"), "d’Évêché de Dorpat");
        assert_eq!(de_place("Guyenne"), "de Guyenne");
        assert_eq!(de_place("Hongrie centrale"), "de Hongrie centrale");
        assert_eq!(de_place("Le Caire"), "du Caire");
        assert_eq!(de_place("Les Landes"), "des Landes");
        assert_eq!(de_place("L’Aigle"), "de l’Aigle");
        assert_eq!(de_place("La Rochelle"), "de La Rochelle");
    }

    #[test]
    fn dates() {
        assert_eq!(date(2026, 9, 24, Some((23, 19))), "24 sept. 2026, 23 h 19");
        assert_eq!(
            date_from_iso("2026-09-24T23:19:05"),
            "24 sept. 2026, 23 h 19"
        );
        assert_eq!(
            date_from_iso("2026-09-24 08:05:00"),
            "24 sept. 2026, 8 h 05"
        );
        assert_eq!(date_from_iso("2026-01-02"), "2 janv. 2026");
        assert_eq!(date_from_iso("pas une date"), "pas une date");
    }
}
