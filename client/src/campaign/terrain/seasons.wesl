// Campaign terrain module `seasons` (VIS-L1, split of terrain.wesl): seasonal colour of the land-cover classes (fields, grass, farm, forest, vine).
// Pure move of the code, no change of behaviour.

import cent_ans_client::campaign::terrain::terrain_params::{season};

// Parcel colour by season. cls: 0 wheat, 1 green crop, 2 ploughed, 3 fallow, 4 meadow, 5 hay,
// 6 vine soil; h: draw of the parcel.
fn season_field(summer: vec3<f32>, cls: i32, h: f32) -> vec3<f32> {
    return season_field_w(summer, cls, h, season.weights);
}

fn season_field_w(summer: vec3<f32>, cls: i32, h: f32, w: vec4<f32>) -> vec3<f32> {
    let young = vec3<f32>(0.10, 0.19, 0.045);
    let plough = vec3<f32>(0.19, 0.135, 0.078);
    let wet_plough = vec3<f32>(0.135, 0.095, 0.06);
    let stubble = vec3<f32>(0.25, 0.20, 0.105);
    let meadow_spring = vec3<f32>(0.085, 0.19, 0.035);
    let meadow_autumn = vec3<f32>(0.11, 0.12, 0.045);
    let meadow_winter = vec3<f32>(0.075, 0.085, 0.048);
    var spring = summer;
    var autumn = summer;
    var winter = summer;
    var ripe = summer;
    if (cls == 0) {
        ripe = summer * vec3<f32>(1.15, 1.06, 0.78);
        spring = select(plough, young, h < 0.8);
        autumn = select(plough, stubble, h < 0.45);
        winter = mix(wet_plough, young, 0.3);
    } else if (cls == 1) {
        spring = mix(young, summer, 0.25);
        autumn = select(mix(stubble, plough, 0.5), plough, h < 0.5);
        winter = wet_plough;
    } else if (cls == 2) {
        spring = plough;
        autumn = plough * 0.9;
        winter = wet_plough;
    } else if (cls == 3) {
        spring = select(mix(summer, young, 0.5), plough, h < 0.5);
        autumn = mix(summer, stubble, 0.4);
        winter = mix(wet_plough, meadow_winter, 0.5);
    } else if (cls == 4) {
        spring = meadow_spring;
        autumn = meadow_autumn;
        winter = meadow_winter;
    } else if (cls == 5) {
        spring = meadow_spring * 1.05;
        autumn = mix(meadow_autumn, stubble, 0.35);
        winter = mix(meadow_winter, stubble * 0.6, 0.4);
    } else {
        autumn = summer * vec3<f32>(1.05, 0.95, 0.9);
        winter = summer * 0.85;
    }
    return spring * w.x + ripe * w.y + autumn * w.z + winter * w.w;
}

fn season_vine_leaf(summer: vec3<f32>) -> vec3<f32> {
    let w = season.weights;
    return summer * 1.15 * w.x + summer * w.y + vec3<f32>(0.24, 0.085, 0.02) * w.z
        + vec3<f32>(0.10, 0.07, 0.045) * w.w;
}

fn season_grass(summer: vec3<f32>) -> vec3<f32> {
    let w = season.weights;
    let spring = summer * vec3<f32>(0.9, 1.18, 0.85);
    let autumn = mix(summer, vec3<f32>(0.13, 0.12, 0.05), 0.5);
    let winter = mix(summer, vec3<f32>(0.08, 0.085, 0.055), 0.65);
    return spring * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

fn season_farm(summer: vec3<f32>) -> vec3<f32> {
    let w = season.weights;
    let spring = mix(summer, vec3<f32>(0.11, 0.15, 0.05), 0.6);
    let autumn = mix(summer, vec3<f32>(0.15, 0.11, 0.06), 0.6);
    let winter = mix(summer, vec3<f32>(0.095, 0.075, 0.05), 0.7);
    return spring * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

fn season_forest(summer: vec3<f32>, grove: f32) -> vec3<f32> {
    let w = season.weights;
    if (w.y > 0.999) {
        return summer;
    }
    let spring = summer * vec3<f32>(1.05, 1.3, 0.9);
    var autumn = summer;
    if (grove < 0.35) {
        autumn = vec3<f32>(0.115, 0.042, 0.010);
    } else if (grove < 0.7) {
        autumn = vec3<f32>(0.12, 0.08, 0.015);
    }
    let winter = select(summer * 0.85, vec3<f32>(0.050, 0.043, 0.036), grove < 0.72);
    return spring * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

fn season_forest_mean(summer: vec3<f32>) -> vec3<f32> {
    let w = season.weights;
    let spring = summer * vec3<f32>(1.05, 1.3, 0.9);
    let autumn = 0.35 * vec3<f32>(0.115, 0.042, 0.010) + 0.35 * vec3<f32>(0.12, 0.08, 0.015) + 0.30 * summer;
    let winter = 0.72 * vec3<f32>(0.050, 0.043, 0.036) + 0.28 * summer * 0.85;
    return spring * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

// Multiplicative season shift (season / summer), bounded: applied on the mean-season colour map
// and materials, which do not know the season.
fn season_ratio(season_col: vec3<f32>, summer_col: vec3<f32>) -> vec3<f32> {
    return clamp(season_col / max(summer_col, vec3<f32>(1e-3)), vec3<f32>(0.35), vec3<f32>(2.6));
}
