//! Names written on the parchment (port of `ParchmentOverlay.refresh` / `_build_realms`): one
//! realm name per faction on its largest connected block of provinces, along the block's main
//! axis, and the short province names (largest first). Pure: testable without rendering.

use std::collections::{HashMap, HashSet};

use bevy::math::Vec2;

/// A province as the realm builder reads it.
#[derive(Debug, Clone)]
pub struct ProvinceIn<'a> {
    pub id: &'a str,
    pub centroid: Vec2,
    pub area: f32,
    pub neighbors: &'a [String],
}

/// A realm name: capitals spaced along `a → b` (map px), centred on `c`, font size `size` at
/// scale 1.
#[derive(Debug, Clone, PartialEq)]
pub struct Realm {
    pub faction: String,
    pub name: String,
    pub a: Vec2,
    pub b: Vec2,
    pub c: Vec2,
    pub size: f32,
}

/// Smallest block area (map px²) that receives a realm name.
pub const MIN_REALM_AREA: f32 = 9000.0;

/// Builds the realm names; `name_of(faction)` gives the display name (upper-cased here).
pub fn build_realms(
    provinces: &[ProvinceIn],
    owner_of: &HashMap<&str, &str>,
    name_of: impl Fn(&str) -> String,
) -> Vec<Realm> {
    let by_id: HashMap<&str, &ProvinceIn> = provinces.iter().map(|p| (p.id, p)).collect();
    let mut seen: HashSet<&str> = HashSet::new();
    // owner -> (area, cluster)
    let mut best: HashMap<&str, (f32, Vec<&ProvinceIn>)> = HashMap::new();
    for province in provinces {
        let owner = owner_of.get(province.id).copied().unwrap_or("");
        if owner.is_empty() || seen.contains(province.id) {
            continue;
        }
        let mut cluster = Vec::new();
        let mut area = 0.0;
        let mut stack = vec![province.id];
        seen.insert(province.id);
        while let Some(current) = stack.pop() {
            let Some(entry) = by_id.get(current) else {
                continue;
            };
            cluster.push(*entry);
            area += entry.area;
            for n in entry.neighbors {
                let nid = n.as_str();
                if by_id.contains_key(nid)
                    && !seen.contains(nid)
                    && owner_of.get(nid).copied().unwrap_or("") == owner
                {
                    let key = by_id[nid].id;
                    seen.insert(key);
                    stack.push(key);
                }
            }
        }
        if area > best.get(owner).map_or(0.0, |b| b.0) {
            best.insert(owner, (area, cluster));
        }
    }
    let mut owners: Vec<&str> = best.keys().copied().collect();
    owners.sort_unstable();
    let mut realms = Vec::new();
    for owner in owners {
        let (area, cluster) = &best[owner];
        if *area < MIN_REALM_AREA {
            continue;
        }
        // Centre and main axis (area-weighted): the name follows the realm's elongation.
        let c = cluster
            .iter()
            .fold(Vec2::ZERO, |acc, e| acc + e.centroid * e.area)
            / *area;
        let (mut xx, mut xy, mut yy) = (0.0, 0.0, 0.0);
        for e in cluster {
            let d = e.centroid - c;
            xx += d.x * d.x * e.area;
            xy += d.x * d.y * e.area;
            yy += d.y * d.y * e.area;
        }
        let limit = 28f32.to_radians();
        let angle = (0.5 * (2.0 * xy).atan2(xx - yy)).clamp(-limit, limit);
        let half = (area.sqrt() * 0.42).clamp(120.0, 520.0);
        let dir = Vec2::from_angle(angle);
        realms.push(Realm {
            faction: owner.to_string(),
            name: name_of(owner).to_uppercase(),
            a: c - dir * half,
            b: c + dir * half,
            c,
            size: (area.sqrt() / 22.0).clamp(14.0, 30.0),
        });
    }
    realms
}

/// Province name without its parenthesised precision (`"Guyenne (Bordelais)"` → `"Guyenne"`).
pub fn short_name(name: &str) -> &str {
    match name.find(" (") {
        Some(i) if i > 0 => &name[..i],
        _ => name,
    }
}

/// Letter layout of an arched realm name (`_draw_arched`): for each glyph, its centre and its
/// rotation (rad), from the screen chord `pa → pb` through `pc`, glyph widths and font size.
pub fn arch_layout(pa: Vec2, pb: Vec2, pc: Vec2, widths: &[f32], size: f32) -> Vec<(Vec2, f32)> {
    let (mut pa, mut pb) = (pa, pb);
    if pb.x < pa.x {
        std::mem::swap(&mut pa, &mut pb);
    }
    let chord = pb - pa;
    let length = chord.length();
    let count = widths.len();
    let mut total: f32 = widths.iter().sum();
    let spacing = ((length - total) / (count.max(2) - 1) as f32).clamp(size * 0.08, size * 0.9);
    total += spacing * count.saturating_sub(1) as f32;
    let dir = chord / length.max(1.0);
    // Screen y grows downwards: this normal points up, so the arch bulges upwards.
    let normal = Vec2::new(dir.y, -dir.x);
    let start = pc - dir * total * 0.5;
    let bend = total * 0.06;
    let mut x = 0.0;
    let mut out = Vec::with_capacity(count);
    for w in widths {
        let t = (x + w * 0.5) / total.max(1.0) * 2.0 - 1.0;
        let pos = start + dir * (x + w * 0.5) + normal * bend * (1.0 - t * t);
        // Half the tangent's tilt (Godot turned the letters the other way, against the arch).
        let angle = dir.y.atan2(dir.x) + (2.0 * bend * t / (total * 0.5).max(1.0)).atan() * 0.5;
        out.push((pos, angle));
        x += w + spacing;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realm_on_the_largest_block_along_its_axis() {
        let n_ab = vec!["b".to_string()];
        let n_ba = vec!["a".to_string()];
        let none: Vec<String> = vec![];
        let provinces = vec![
            ProvinceIn {
                id: "a",
                centroid: Vec2::new(0.0, 0.0),
                area: 6000.0,
                neighbors: &n_ab,
            },
            ProvinceIn {
                id: "b",
                centroid: Vec2::new(400.0, 0.0),
                area: 6000.0,
                neighbors: &n_ba,
            },
            ProvinceIn {
                id: "c",
                centroid: Vec2::new(2000.0, 0.0),
                area: 5000.0,
                neighbors: &none,
            },
        ];
        let owners: HashMap<&str, &str> = [("a", "fr"), ("b", "fr"), ("c", "fr")].into();
        let realms = build_realms(&provinces, &owners, |_| "France".into());
        assert_eq!(realms.len(), 1);
        let realm = &realms[0];
        assert_eq!(realm.name, "FRANCE");
        assert!((realm.c - Vec2::new(200.0, 0.0)).length() < 1e-3);
        assert!((realm.a.y - realm.c.y).abs() < 1e-3, "horizontal axis");
        // Too small alone: no name.
        let owners: HashMap<&str, &str> = [("c", "en")].into();
        assert!(build_realms(&provinces, &owners, |_| "X".into()).is_empty());
    }

    #[test]
    fn arch_letters_follow_the_chord_and_bulge_up() {
        let letters = arch_layout(
            Vec2::new(0.0, 100.0),
            Vec2::new(300.0, 100.0),
            Vec2::new(150.0, 100.0),
            &[20.0; 5],
            20.0,
        );
        assert_eq!(letters.len(), 5);
        assert!(letters[0].0.x < letters[4].0.x);
        assert!(letters[2].0.y < letters[0].0.y, "middle letter higher");
        assert!(
            letters[0].1 < 0.0 && letters[4].1 > 0.0,
            "tops lean outwards"
        );
        assert_eq!(short_name("Guyenne (Bordelais)"), "Guyenne");
    }
}
