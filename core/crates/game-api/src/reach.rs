//! Mask of the cells an army reaches (the "movement bubble"), read-only view. Same content as
//! `get_reachable_area` of the Godot bridge: one RGB8 texel per navigation-grid cell, cropped to
//! the bounding box of the area plus a margin. R = 255 inside this turn's area, G = cost /
//! budget x 255, B = 255 inside the two-turn area (this turn's included). Thin gaps (rivers crossed
//! by a bridge further on) are closed for display only. Costs and zones of control are those of
//! the real march (`CampaignState::reachable_cells`).

use data_model::GameData;
use serde::Serialize;
use sim_campaign::{ArmyId, CampaignState};

/// Empty cells kept around the bubble in the mask (smooth contour).
pub const MASK_MARGIN: u32 = 2;
/// Bytes per texel of the mask (RGB8).
pub const MASK_STRIDE: usize = 3;
/// Widest gap (cells) closed between two reachable cells.
const THIN_GAP: usize = 2;

#[derive(Debug, Clone, Serialize)]
pub struct ReachMask {
    /// Map pixels covered: top-left corner and extent.
    pub origin: [f32; 2],
    pub size: [f32; 2],
    pub width: u32,
    pub height: u32,
    /// Side of a grid cell in map pixels.
    pub cell_px: f32,
    /// RGB8, `width x height` texels, row-major.
    pub bytes: Vec<u8>,
    /// Cells reached this turn, and only next turn.
    pub cells: usize,
    pub next_cells: usize,
    pub budget: u32,
    pub next_budget: u32,
}

/// Mask of `army`, `None` for an unknown army.
pub fn reach_mask(data: &GameData, state: &CampaignState, army: &ArmyId) -> Option<ReachMask> {
    let entry = state.army(army)?;
    let grid = data.navgrid();
    let area = state.reachable_cells(data, army);
    let start = state.army_cell(data, entry);
    let (mut x0, mut y0, mut x1, mut y1) = (
        u32::from(start.x),
        u32::from(start.y),
        u32::from(start.x),
        u32::from(start.y),
    );
    for cell in area
        .this_turn
        .iter()
        .map(|(c, _)| c)
        .chain(area.next_turn.iter())
    {
        x0 = x0.min(u32::from(cell.x));
        y0 = y0.min(u32::from(cell.y));
        x1 = x1.max(u32::from(cell.x));
        y1 = y1.max(u32::from(cell.y));
    }
    let x0 = x0.saturating_sub(MASK_MARGIN);
    let y0 = y0.saturating_sub(MASK_MARGIN);
    let x1 = (x1 + MASK_MARGIN).min(grid.width.saturating_sub(1));
    let y1 = (y1 + MASK_MARGIN).min(grid.height.saturating_sub(1));
    let width = x1 - x0 + 1;
    let height = y1 - y0 + 1;
    let budget = area.budget.max(1);
    let texel = |cell: &sim_campaign::navigation::Cell| {
        ((u32::from(cell.y) - y0) * width + (u32::from(cell.x) - x0)) as usize * MASK_STRIDE
    };
    let mut bytes = vec![0u8; (width * height) as usize * MASK_STRIDE];
    for (cell, cost) in &area.this_turn {
        let index = texel(cell);
        bytes[index] = 255;
        bytes[index + 1] = ((u64::from(*cost) * 255) / u64::from(budget)).min(255) as u8;
        bytes[index + 2] = 255;
    }
    for cell in &area.next_turn {
        bytes[texel(cell) + 2] = 255;
    }
    close_thin_gaps(&mut bytes, width as usize, height as usize, 0, Some(1));
    close_thin_gaps(&mut bytes, width as usize, height as usize, 2, None);
    let scale = grid.scale as f32;
    Some(ReachMask {
        origin: [x0 as f32 * scale, y0 as f32 * scale],
        size: [width as f32 * scale, height as f32 * scale],
        width,
        height,
        cell_px: scale,
        bytes,
        cells: area.this_turn.len(),
        next_cells: area.next_turn.len(),
        budget: area.budget,
        next_budget: area.next_budget,
    })
}

/// Display only: fills the gaps of at most `THIN_GAP` cells between two reachable cells of a row
/// or a column, so that the bubble reads as one area instead of being striped by every river. A
/// texel is inside when its `channel` is 255; a filled one takes the smaller `cost_channel`
/// value of its two neighbours.
pub fn close_thin_gaps(
    bytes: &mut [u8],
    width: usize,
    height: usize,
    channel: usize,
    cost_channel: Option<usize>,
) {
    let at = |x: usize, y: usize| (y * width + x) * MASK_STRIDE;
    let inside = |bytes: &[u8], x: usize, y: usize| bytes[at(x, y) + channel] == 255;
    let cost = |bytes: &[u8], x: usize, y: usize| cost_channel.map_or(0, |c| bytes[at(x, y) + c]);
    let mut fills: Vec<(usize, u8)> = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if inside(bytes, x, y) {
                continue;
            }
            let mut best: Option<u8> = None;
            for (dx, dy) in [(1usize, 0usize), (0, 1)] {
                for before in 1..=THIN_GAP {
                    for after in 1..=THIN_GAP {
                        if before + after > THIN_GAP + 1 {
                            continue;
                        }
                        let (Some(bx), Some(by)) =
                            (x.checked_sub(before * dx), y.checked_sub(before * dy))
                        else {
                            continue;
                        };
                        let (ax, ay) = (x + after * dx, y + after * dy);
                        if ax >= width || ay >= height {
                            continue;
                        }
                        if inside(bytes, bx, by) && inside(bytes, ax, ay) {
                            let c = cost(bytes, bx, by).max(cost(bytes, ax, ay));
                            best = Some(best.map_or(c, |b| b.min(c)));
                        }
                    }
                }
            }
            if let Some(c) = best {
                fills.push((at(x, y), c));
            }
        }
    }
    for (index, c) in fills {
        bytes[index + channel] = 255;
        if let Some(cost_channel) = cost_channel {
            bytes[index + cost_channel] = c;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_of_the_main_army() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let id = sim_campaign::test_support::main_army(&state, "fac_france");
        let mask = reach_mask(data, &state, &id).unwrap();
        assert_eq!(
            mask.bytes.len(),
            (mask.width * mask.height) as usize * MASK_STRIDE
        );
        let this: usize = mask.bytes.chunks(3).filter(|t| t[0] == 255).count();
        assert!(this >= mask.cells, "gap closing only adds texels");
        let two_turns: usize = mask.bytes.chunks(3).filter(|t| t[2] == 255).count();
        assert!(two_turns >= this);
        assert!(mask.size[0] > 0.0 && mask.cell_px > 0.0);
        assert!(reach_mask(data, &state, &ArmyId::parse("army_9999").unwrap()).is_none());
    }

    #[test]
    fn a_river_gap_is_closed() {
        // Two inside texels with one empty between them on a row of 4 texels.
        let mut bytes = vec![0u8; 4 * MASK_STRIDE];
        for x in [0usize, 2] {
            bytes[x * MASK_STRIDE] = 255;
            bytes[x * MASK_STRIDE + 1] = 100;
        }
        close_thin_gaps(&mut bytes, 4, 1, 0, Some(1));
        assert_eq!(bytes[MASK_STRIDE], 255);
        assert_eq!(bytes[MASK_STRIDE + 1], 100);
        assert_eq!(bytes[3 * MASK_STRIDE], 0, "outside stays empty");
    }
}
