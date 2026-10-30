// Campaign terrain module `fields_paint` (VIS-L1, split of terrain.wesl): painted fields applied over the ground colour (fields_apply).
// Pure move of the code, no change of behaviour.

import cent_ans_client::campaign::terrain::ground_detail::gd_parcel_k;
import cent_ans_client::campaign::terrain::ground_detail::gd_warp;
import cent_ans_client::campaign::terrain::ground_layers::{terroir_at};
import cent_ans_client::campaign::terrain::noise::{luma, vnoise};
import cent_ans_client::campaign::terrain::parcels::{PcFrame, field_row, hb_far_patch, hb_hash, hb_maille, hb_maille_far, hb_patch_color, hb_relative, hb_rot, hb_sample, hb_sample_normal, hb_voronoi, map_cell, pc_cell_of, pc_class, pc_hash, season_field_form, tex_mean};
import cent_ans_client::campaign::terrain::terrain_params::{fields, gd, ground, params, season};

const HB_SEASON_STRENGTH = 0.8;

struct FieldsOut {
    color: vec3<f32>,
    dn: vec2<f32>,
};

// Port of `hb_apply` (hb_ground.gdshaderinc) on `col` (the colormap-blended ground): near and
// mid view, the parcels of the landscape drawn with their own materials (`tx_campaign_parcels`:
// crop of the parcel, wild soil of the unfarmed ones, canopy, rock), their season, hedges and
// rims; far, the patchwork of soles and terroirs. Crops follow the biome / landscape table,
// relief and water decide where they grow (GR, ADR 0352).
fn fields_apply(p: vec2<f32>, uv: vec2<f32>, footprint_in: f32, land: f32, open_w: f32, forest_w: f32,
        rock_w: f32, farm_share: f32, slope: f32, grad: vec2<f32>, alt: f32, k_wild: vec3<f32>,
        k_forest: vec3<f32>, col_in: vec3<f32>) -> FieldsOut {
    var out = FieldsOut(col_in, vec2<f32>(0.0));
    // Derivatives first (uniform control flow).
    let dpx = dpdx(p);
    let dpy = dpdy(p);
    // Godot's footprint is the screen derivative (`max(length(dFdx(p)), length(dFdy(p)))`), larger
    // than the distance footprint on grazing ground: the patch colour of the crops comes in there.
    let footprint = max(footprint_in, max(length(dpx), length(dpy)));
    var col = col_in;
    let mpp = max(ground.micro.w, 1.0);
    let fade = smoothstep(fields.v0.x, fields.v0.y, footprint);
    var w_bare = fields.v5.x * (1.0 - fade) * land;
    var w_far = 0.0;
    if (fields.v2.y > 0.0) {
        w_far = fields.v2.w * fade * (1.0 - smoothstep(fields.v3.x, fields.v3.y, footprint)) * land;
    }
    if (w_bare <= 0.001 && w_far <= 0.001) {
        return out;
    }
    let region_cells = fields.v6.x;
    let reg = hb_voronoi(p / region_cells + vec2<f32>(vnoise(p * 0.05), vnoise(p * 0.05 + 9.0)) * 0.6);
    let h_dom = hb_hash(reg.id + 9.9);
    let ang_r = hb_hash(reg.id + 41.0) * 3.1416;
    let origin = reg.site * region_cells;
    let map_px = params.map.xy;
    let cell0 = map_cell(p / map_px);
    let row0 = field_row(cell0);
    // VIS-R3b: the form of the landscape sets the stretch, the cell size and the warp of the grid.
    let forms_on = fields.pc0.y > 0.5;
    let form0 = select(0u, u32(fields.rows_b[row0].z + 0.5), forms_on);
    let fa = fields.forms_a[form0];
    var strip = clamp(fields.rows_a[row0].y, 1.0, 2.2);
    if (forms_on && fa.y > 0.0) {
        strip = mix(fa.x, fa.y, hb_hash(reg.id + 13.7));
    }
    let cell_px = max(fields.rows_a[row0].x, 50.0) * fields.v2.x * select(1.0, fa.w, forms_on) / mpp;
    let warp_k = select(1.0, fa.z, forms_on);
    // VIS-L1: a parcel narrower than a few screen pixels is not drawn one by one (the combed
    // ground): its own colour, grain and hedges fade into the ground colour, the far patchwork
    // (coarser levels) takes over.
    if (forms_on && fields.pc0.w > 0.0) {
        w_bare = w_bare * smoothstep(fields.pc0.z, fields.pc0.w, cell_px / max(footprint, 1e-6));
    }
    let rot_f = hb_rot(ang_r);
    var uu = rot_f * (p - origin) / cell_px;
    uu.x = uu.x / strip;
    uu = uu + ((vec2<f32>(vnoise(uu * 0.9), vnoise(uu * 0.9 + 17.3)) - 0.5) * 0.55
        + (vec2<f32>(vnoise(uu * 3.3 + 7.7), vnoise(uu * 3.3 + 29.1)) - 0.5) * 0.14) * warp_k;
    uu = uu + gd_warp(gd, uu) * warp_k;  // VQ cground2: curved boundaries
    uu = uu + reg.id * 37.0;
    let frame = PcFrame(transpose(rot_f), strip, cell_px, origin, reg.id * 37.0, form0);
    let parcel = pc_cell_of(uu, form0, strip);
    let pc = transpose(rot_f) * (vec2<f32>((parcel.site.x - reg.id.x * 37.0) * strip, parcel.site.y - reg.id.y * 37.0) * cell_px) + origin;
    // The parcel is one biome / landscape: read at its centre.
    let cell_c = map_cell(pc / map_px);
    let row = field_row(cell_c);
    let form_p = select(0u, u32(fields.rows_b[row].z + 0.5), forms_on);
    let hedge_k = fields.rows_a[row].z;
    var presence = fields.rows_a[row].w;
    var vine_k = 0.0;
    var meadow_k = 0.0;
    if (fields.v6.y > 0.5) {
        let south = -grad.y / max(length(grad), 1e-6);
        vine_k = fields.rows_b[row].y * smoothstep(fields.v7.z, fields.v7.z * 2.0, slope)
            * (1.0 - smoothstep(fields.v7.w, fields.v7.w * 1.4, slope))
            * smoothstep(fields.v8.x, fields.v8.x + 0.35, south);
        meadow_k = fields.v8.z * (1.0 - smoothstep(fields.v8.y * 0.5, fields.v8.y, cell0.river_px))
            * (1.0 - smoothstep(0.06, 0.18, slope));
        let relief_k = (1.0 - smoothstep(fields.v6.z, fields.v6.w, alt))
            * max(1.0 - smoothstep(fields.v7.x, fields.v7.y, slope), vine_k);
        presence = presence * relief_k;
    }
    let farm_frac = clamp(max(farm_share, fields.v4.w) * presence, 0.0, 1.0);
    var far_on = w_far > 0.001;
    @if(PERF_TNOFAR) {
        far_on = false;
    }
    if (far_on) {
        let far_col = hb_far_patch(uu, footprint, footprint / cell_px, row, h_dom, vine_k, meadow_k, frame);
        col = mix(col, hb_relative(col, far_col.rgb), w_far * open_w * farm_frac * far_col.a);
    }
    out.color = col;
    if (w_bare <= 0.001) {
        return out;
    }
    var has_tex = fields.tex_flags.z > 0.5 && fields.rows_e[row].w > 0.5;
    @if(PERF_TNOHBTEX) {
        has_tex = false;
    }
    let sw = season.weights;
    // Unfarmed parcel: the two wild soils of the row, mixed by a slow noise.
    let wild_mix = smoothstep(0.35, 0.65, vnoise(p * 0.35 + 3.7));
    var open_col = col;
    if (has_tex) {
        let wd = fields.rows_d[row];
        open_col = mix(hb_sample(wd.x, wd.y, p, dpx, dpy, ang_r * 0.3), hb_sample(wd.z, wd.w, p, dpx, dpy, 1.1), wild_mix);
    }
    open_col = open_col * pow(k_wild, vec3<f32>(HB_SEASON_STRENGTH));  // TB1
    var hedge_mask = 0.0;
    var nrm = vec2<f32>(0.0);
    var gd_k = vec3<f32>(1.0);
    let farmed = hb_hash(parcel.id + 71.7) < max(farm_share, fields.v4.w) * presence;
    if (farmed) {
        var key = parcel.id;
        var sole = false;
        if (fields.v2.y > 0.0 && hb_hash(parcel.id + 5.1) < fields.v2.z) {
            key = pc_cell_of(parcel.site * fields.v2.y, form_p, strip).id;
            sole = true;
        }
        var ring_r = -1.0;
        if (forms_on && fields.forms_b[form_p].z > 0.0) {
            ring_r = terroir_at(pc).r;
        }
        let m = hb_maille(key, row, h_dom, vine_k, meadow_k, ring_r, form_p);
        var layer = -1.0;
        var tile = 120.0;
        if (m >= 0) {
            let st = fields.slot_tex[row * 12u + u32(m)];
            layer = st.x;
            tile = st.y;
        } else if (m == -2) {
            layer = fields.tex_extra.z;
            tile = fields.tex_extra.w;
        } else if (m == -3) {
            layer = fields.tex_extra.x;
            tile = fields.tex_extra.y;
        }
        if (m != -1) {
            let far = hb_maille_far(m, row);
            let cls_crop = i32(far.a + 0.5);
            // Class of the parcel in the dominant season (shares of its form, parcel.rs).
            let sw_d = season.weights;
            var s_dom = 0u;
            if (sw_d.y > sw_d[s_dom]) {
                s_dom = 1u;
            }
            if (sw_d.z > sw_d[s_dom]) {
                s_dom = 2u;
            }
            if (sw_d.w > sw_d[s_dom]) {
                s_dom = 3u;
            }
            let hq = pc_hash(parcel.id.x, parcel.id.y + 211.0);
            let hr = pc_hash(parcel.id.x + 5.0, parcel.id.y + 97.0);
            let cls = pc_class(form_p, s_dom, cls_crop, hq, hr);
            let field_ang = ang_r + select(1.5708, 0.0, hb_hash(parcel.id + 5.3) < 0.85);
            let textured = has_tex && layer >= 0.0;
            var mean = vec3<f32>(max(luma(col), 1e-3));
            if (textured) {
                open_col = hb_sample(layer, tile, p, dpx, dpy, field_ang);
                nrm = hb_sample_normal(layer, tile, p, dpx, dpy, field_ang);
                mean = tex_mean(layer);
            } else {
                open_col = col;
            }
            // Each parcel has its own nuance (maturity, soil, exposure).
            let j1 = hb_hash(parcel.id + 3.3) - 0.5;
            let j2 = hb_hash(parcel.id + 9.1) - 0.5;
            open_col = open_col * (1.0 + 2.0 * fields.v1.y * j1) * vec3<f32>(1.0 + fields.v1.z * j2, 1.0, 1.0 - fields.v1.z * j2);
            // VIS-R3b: anisotropic grain of the ploughland (furrows), resolved only when the
            // pitch spans a few pixels; streaks along the strip, finer across.
            let fc = fields.forms_c[form_p];
            if (forms_on && fc.z > 0.0 && cls <= 3) {
                let px_f = max(footprint * mpp, 0.01);
                let resolved = smoothstep(1.0, 3.0, fc.w / px_f);
                if (resolved > 0.001) {
                    let q = (rot_f * (p - origin)) * mpp;
                    let n = vnoise(vec2<f32>(q.x / (fc.w * 9.0), q.y / fc.w) + parcel.id * 3.7);
                    open_col = open_col * (1.0 + fc.z * resolved * (n - 0.5) * 2.0);
                }
            }
            // TB1: season of the parcel; the material is of a mean season (summer gilds the wheat,
            // the other seasons take the crop's season colour and keep the grain).
            if (cls == 0) {
                open_col = open_col * mix(vec3<f32>(1.0), season.summer_gold.rgb, sw.y * 0.8);
            }
            if (sw.y < 0.999) {
                let grain_s = clamp(luma(open_col) / max(luma(mean), 1e-3), 0.6, 1.5);
                let season_col = season_field_form(far.rgb, cls_crop, hb_hash(parcel.id + 61.3), hq, hr, form_p);
                open_col = mix(open_col, season_col * grain_s, 0.8 * (1.0 - sw.y));
            }
            // ADR 0253: from mid distance the parcel takes the colour of its crop, keeping the
            // grain of the material (luminance relative to its mean).
            let patch_k = fields.v0.z * smoothstep(fields.v0.w, fields.v1.x, footprint);
            if (patch_k > 0.001) {
                let lumv = 1.0 + 2.0 * fields.v1.y * (hb_hash(parcel.id + 3.3) - 0.5);
                var far_col = season_field_form(far.rgb, cls_crop, hb_hash(parcel.id + 61.3), hq, hr, form_p) * lumv;
                if (sole) {
                    far_col = hb_patch_color(key, far, form_p);
                }
                let grain_l = clamp(luma(open_col) / max(luma(mean), 1e-3), 0.7, 1.35);
                open_col = mix(open_col, far_col * grain_l, patch_k);
            }
            @if(!PERF_TNOGDK)
            gd_k = gd_parcel_k(gd, open_col, p, parcel.id, field_ang, cls, footprint, mpp, season.weights,
                parcel.edge * cell_px * mpp, hedge_k);
            // Hedges and banks on the edges of the cultivated parcels.
            let edge_m = parcel.edge * cell_px * mpp;
            let px_m = footprint * mpp;
            let hedge_w = fields.v5.w * hedge_k;
            var hedge = 0.0;
            if (hedge_k > 0.01) {
                hedge = (1.0 - smoothstep(0.0, max(px_m, hedge_w), edge_m - hedge_w))
                    * clamp(2.0 * hedge_w / max(px_m, 1e-3), max(0.35, fields.v4.z), 1.0);
            }
            // VIS-R3b: the hedge of the bocage keeps a width of a few screen pixels.
            if (forms_on && hedge_k > 0.01 && fc.x > 0.0) {
                let near_k = 1.0 - smoothstep(fc.y * 0.6, fc.y, footprint);
                let half_px = 0.5 * fc.x * near_k;
                hedge = max(hedge, near_k * (1.0 - smoothstep(px_m * max(half_px - 0.5, 0.0), px_m * (half_px + 0.5), edge_m)));
            }
            let balk = 1.0 - smoothstep(1.5, 1.5 + px_m, edge_m);
            open_col = mix(open_col, open_col * 0.82, balk * 0.6);
            let rim = 1.0 - smoothstep(0.0, max(footprint * 1.4, 0.02), parcel.edge * cell_px - 0.006);
            open_col = open_col * (1.0 - 0.28 * rim * (1.0 - smoothstep(0.1, 0.3, footprint)));
            open_col = mix(open_col, fields.hedge.rgb, hedge * 0.8);
            hedge_mask = hedge;
            nrm = nrm * (1.0 - hedge);
        }
    }
    var canopy_col = col;
    var rock_col = col;
    if (has_tex) {
        let cr = fields.rows_c[row];
        canopy_col = hb_sample(cr.x, cr.y, p, dpx, dpy, 0.4) * pow(k_forest, vec3<f32>(HB_SEASON_STRENGTH));
        rock_col = hb_sample(cr.z, cr.w, p, dpx, dpy, 0.0);
    }
    // Rock: rock share of the splat, more on the steep slopes.
    let rock_share = clamp(rock_w + smoothstep(0.35, 0.8, slope) * (1.0 - forest_w), 0.0, 1.0);
    let total = max(open_w + forest_w + rock_share, 1e-3);
    var hb_col = (open_col * open_w + canopy_col * forest_w + rock_col * rock_share) / total;
    if (has_tex) {
        hb_col = hb_col * fields.rows_e[row].rgb;
    }
    // Hue: a share of the colour map's (consistent with the far view), luminance in the band.
    let lum_hb = luma(hb_col);
    let lum_c = max(luma(col), 1e-3);
    hb_col = mix(hb_col, col * (lum_hb / lum_c), fields.v5.z);
    hb_col = hb_relative(col, hb_col);
    // VQ cground: parcel tone, crop stage and strips, outside the luminance band of `hb_relative`.
    hb_col = hb_col * mix(vec3<f32>(1.0), gd_k, open_w / total);
    out.color = mix(col, hb_col, w_bare);
    out.dn = nrm * w_bare * open_w / total * ground.tiling.w * 0.6;
    return out;
}
