"""Icon choices: one game-icons.net SVG per game identifier (F2).

Each entry maps an identifier used by the game (a ``data/`` id such as
``unit_knights`` or ``bld_market``, or a UI id such as ``hud_treasury``) to a
source path ``<author>/<name>`` in https://github.com/game-icons/icons and a
category used by ``IconLibrary`` for fallbacks. All icons are CC BY 3.0; the
author slug is the first path component and is credited in ``CREDITS.md``.

Categories and their fallback ids (``cat_<category>``) are listed in
``FALLBACKS``; an id missing from the table falls back on its category icon.
"""

from __future__ import annotations

# Display names of the game-icons.net authors (slug -> credited name).
AUTHORS: dict[str, str] = {
    "caro-asercion": "Caro Asercion",
    "carl-olsen": "Carl Olsen",
    "cathelineau": "Cathelineau",
    "delapouite": "Delapouite",
    "faithtoken": "Faithtoken",
    "heavenly-dog": "HeavenlyDog",
    "lorc": "Lorc",
    "skoll": "Skoll",
}

# Category -> fallback id (must be an entry of ICONS).
FALLBACKS: dict[str, str] = {
    "unit": "cat_unit",
    "building": "cat_building",
    "resource": "cat_resource",
    "technology": "cat_technology",
    "class": "cat_class",
    "gauge": "cat_gauge",
    "hud": "cat_hud",
    "branch": "cat_branch",
    "trait": "cat_trait",
    "skill": "cat_skill",
    "default": "cat_default",
}

# id -> (source "<author>/<name>", category)
ICONS: dict[str, tuple[str, str]] = {
    # --- Fallbacks par catégorie ---
    "cat_unit": ("lorc/crossed-swords", "unit"),
    "cat_building": ("lorc/stone-tower", "building"),
    "cat_resource": ("delapouite/wooden-crate", "resource"),
    "cat_technology": ("lorc/gears", "technology"),
    "cat_class": ("delapouite/village", "class"),
    "cat_gauge": ("lorc/scales", "gauge"),
    "cat_hud": ("lorc/tied-scroll", "hud"),
    "cat_branch": ("lorc/laurels", "branch"),
    "cat_trait": ("lorc/drama-masks", "trait"),
    "cat_skill": ("lorc/laurels", "skill"),
    "cat_default": ("lorc/tied-scroll", "default"),
    # --- Types d'unités (data/unit_types) ---
    "unit_bombard": ("lorc/cannon", "unit"),
    "unit_crossbowmen": ("carl-olsen/crossbow", "unit"),
    "unit_flemish_pikemen": ("delapouite/pikeman", "unit"),
    "unit_genoese_crossbowmen": ("lorc/arrows-shield", "unit"),
    "unit_knights": ("skoll/mounted-knight", "unit"),
    "unit_longbowmen": ("lorc/bowman", "unit"),
    "unit_mangonel": ("heavenly-dog/catapult", "unit"),
    "unit_men_at_arms_foot": ("cathelineau/swordman", "unit"),
    "unit_mounted_archers": ("caro-asercion/cloaked-figure-on-horseback", "unit"),
    "unit_mounted_sergeants": ("delapouite/horse-head", "unit"),
    "unit_siege_tower": ("delapouite/siege-tower", "unit"),
    "unit_trebuchet": ("delapouite/trebuchet", "unit"),
    "unit_urban_militia": ("lorc/halberd", "unit"),
    # Catégories d'unités (repli en bataille : clé `render`/`category`).
    "unit_category_infantry": ("lorc/crossed-swords", "unit"),
    "unit_category_ranged": ("lorc/bowman", "unit"),
    "unit_category_archer": ("lorc/bowman", "unit"),
    "unit_category_cavalry": ("skoll/mounted-knight", "unit"),
    "unit_category_siege": ("delapouite/trebuchet", "unit"),
    "unit_category_tower": ("delapouite/siege-tower", "unit"),
    "unit_category_ram": ("skoll/siege-ram", "unit"),
    # --- Bâtiments (data/buildings) ---
    "bld_abbey": ("delapouite/abbot-meeple", "building"),
    "bld_archery_butts": ("lorc/archery-target", "building"),
    "bld_armoury": ("delapouite/chest-armor", "building"),
    "bld_artillery_bastion": ("delapouite/military-fort", "building"),
    "bld_castle": ("lorc/castle", "building"),
    "bld_cathedral": ("lorc/gothic-cross", "building"),
    "bld_collegiate_church": ("delapouite/church", "building"),
    "bld_counting_house": ("delapouite/abacus", "building"),
    "bld_fair": ("delapouite/medieval-pavilion", "building"),
    "bld_forge": ("lorc/anvil", "building"),
    "bld_guild_hall": ("delapouite/hanging-sign", "building"),
    "bld_hotel_dieu": ("lorc/hospital-cross", "building"),
    "bld_market": ("delapouite/shop", "building"),
    "bld_muster_field": ("delapouite/medieval-barracks", "building"),
    "bld_palisade": ("delapouite/palisade", "building"),
    "bld_parish_church": ("delapouite/church", "building"),
    "bld_port": ("delapouite/harbor-dock", "building"),
    "bld_scriptorium": ("delapouite/scroll-quill", "building"),
    "bld_siege_workshop": ("delapouite/hand-saw", "building"),
    "bld_stables": ("delapouite/stable", "building"),
    "bld_stone_walls": ("delapouite/stone-wall", "building"),
    "bld_tin_blowing_house": ("delapouite/furnace", "building"),
    "bld_university": ("delapouite/graduate-cap", "building"),
    "bld_vineyard_press": ("delapouite/barrel", "building"),
    "bld_water_mill": ("caro-asercion/water-mill", "building"),
    "bld_water_supply": ("delapouite/well", "building"),
    "bld_weaving_workshop": ("caro-asercion/spinning-wheel", "building"),
    "bld_windmill": ("delapouite/windmill", "building"),
    # Catégories de bâtiments.
    "building_category_production": ("lorc/anvil", "building"),
    "building_category_commerce": ("delapouite/shop", "building"),
    "building_category_military": ("delapouite/medieval-barracks", "building"),
    "building_category_religious": ("delapouite/church", "building"),
    "building_category_sanitary": ("lorc/hospital-cross", "building"),
    "building_category_fortification": ("delapouite/stone-wall", "building"),
    # --- Ressources (data/resources) ---
    "res_cloth": ("delapouite/rolled-cloth", "resource"),
    "res_fish": ("delapouite/double-fish", "resource"),
    "res_iron": ("lorc/metal-bar", "resource"),
    "res_salt": ("lorc/powder", "resource"),
    "res_stone": ("delapouite/stone-pile", "resource"),
    "res_tin": ("faithtoken/ore", "resource"),
    "res_wheat": ("lorc/wheat", "resource"),
    "res_wine": ("lorc/grapes", "resource"),
    "res_wood": ("delapouite/wood-pile", "resource"),
    "res_wool": ("delapouite/wool", "resource"),
    # --- Régimes alimentaires (data/diets, S3) ---
    "diet_bread_pottage": ("lorc/cauldron", "resource"),
    "diet_dairy": ("lorc/cheese-wedge", "resource"),
    "diet_lenten_fish": ("delapouite/fish-smoking", "resource"),
    "diet_meat_salting": ("delapouite/bacon", "resource"),
    "diet_pulses": ("delapouite/peas", "resource"),
    "diet_spiced_table": ("lorc/hot-spices", "resource"),
    "diet_wine_bread": ("lorc/wine-glass", "resource"),
    # --- Technologies (data/technologies) ---
    "tech_artillery_fortification": ("heavenly-dog/defensive-wall", "technology"),
    "tech_bombards": ("lorc/cannon", "technology"),
    "tech_bookkeeping": ("lorc/quill-ink", "technology"),
    "tech_brigandine": ("lorc/armor-vest", "technology"),
    "tech_coat_of_plates": ("lorc/breastplate", "technology"),
    "tech_compagnies_d_ordonnance": ("delapouite/knight-banner", "technology"),
    "tech_crossbow_windlass": ("carl-olsen/crossbow", "technology"),
    "tech_dismounted_tactics": ("lorc/spears", "technology"),
    "tech_double_entry": ("delapouite/abacus", "technology"),
    "tech_field_artillery": ("lorc/cannon-shot", "technology"),
    "tech_francs_archers": ("lorc/bowman", "technology"),
    "tech_full_plate": ("lorc/visored-helm", "technology"),
    "tech_gothic_flamboyant": ("lorc/gothic-cross", "technology"),
    "tech_gunpowder": ("delapouite/powder-bag", "technology"),
    "tech_hand_cannon_drill": ("skoll/musket", "technology"),
    "tech_handgonnes": ("lorc/gunshot", "technology"),
    "tech_hanseatic_trade": ("lorc/galleon", "technology"),
    "tech_hospital_reform": ("lorc/bandage-roll", "technology"),
    "tech_letters_of_credit": ("lorc/wax-seal", "technology"),
    "tech_longbow_drill": ("lorc/target-arrows", "technology"),
    "tech_masonry": ("lorc/stone-block", "technology"),
    "tech_paper_mills": ("lorc/papers", "technology"),
    "tech_pavise": ("lorc/arrows-shield", "technology"),
    "tech_printing_press": ("lorc/open-book", "technology"),
    "tech_quarantine": ("delapouite/plague-doctor-profile", "technology"),
    "tech_royal_taxation": ("lorc/crown-coin", "technology"),
    "tech_siege_engineering": ("delapouite/trebuchet", "technology"),
    "tech_standing_companies": ("delapouite/barracks-tent", "technology"),
    "tech_three_field_rotation": ("delapouite/plow", "technology"),
    "tech_universities": ("delapouite/diploma", "technology"),
    "tech_urban_sanitation": ("delapouite/broom", "technology"),
    "tech_water_mills": ("caro-asercion/water-mill", "technology"),
    "tech_windmills": ("delapouite/windmill", "technology"),
    # Familles (branches) de technologies.
    "tech_branch_military": ("lorc/crossed-swords", "technology"),
    "tech_branch_civil": ("lorc/quill-ink", "technology"),
    "tech_branch_medicine": ("delapouite/healing", "technology"),
    # H4 : arbre de la médecine et bâtiments sanitaires.
    "tech_herb_garden": ("delapouite/herbs-bundle", "technology"),
    "tech_humoral_theory": ("lorc/drop", "technology"),
    "tech_regimen_sanitatis": ("lorc/scroll-unfurled", "technology"),
    "tech_willow_bark": ("lorc/falling-leaf", "technology"),
    "tech_barber_surgeons": ("lorc/scalpel", "technology"),
    "tech_theriac": ("lorc/potion-ball", "technology"),
    "tech_montpellier": ("delapouite/graduate-cap", "technology"),
    "tech_soporific_sponge": ("lorc/sleepy", "technology"),
    "tech_plague_consilia": ("lorc/leeching-worm", "technology"),
    "tech_chauliac_surgery": ("lorc/scalpel-strike", "technology"),
    "tech_leprosaria": ("delapouite/hospital", "technology"),
    "tech_aqua_vitae": ("lorc/round-bottom-flask", "technology"),
    "bld_herb_garden": ("delapouite/herbs-bundle", "building"),
    "bld_apothecary": ("delapouite/medicines", "building"),
    # --- Classes sociales ---
    "class_peasants": ("delapouite/farmer", "class"),
    "class_burghers": ("caro-asercion/medieval-village-01", "class"),
    "class_clergy": ("lorc/prayer", "class"),
    "class_nobility": ("lorc/crown", "class"),
    # --- Jauges ---
    "gauge_unrest": ("lorc/fist", "gauge"),
    "gauge_health": ("delapouite/healing", "gauge"),
    "gauge_wealth": ("delapouite/coins", "gauge"),
    "gauge_goods_satisfaction": ("lorc/swap-bag", "gauge"),
    "gauge_devastation": ("delapouite/castle-ruins", "gauge"),
    "gauge_population": ("delapouite/village", "gauge"),
    "gauge_morale": ("lorc/flying-flag", "gauge"),
    "gauge_supply": ("delapouite/bread", "gauge"),
    "gauge_movement": ("lorc/boot-prints", "gauge"),
    "gauge_strength": ("lorc/crossed-swords", "gauge"),
    # --- Barre du haut (HUD) ---
    "hud_treasury": ("skoll/open-treasure-chest", "hud"),
    "hud_income": ("delapouite/two-coins", "hud"),
    "hud_research": ("lorc/open-book", "hud"),
    "hud_codex": ("lorc/scroll-unfurled", "hud"),
    "hud_season_spring": ("lorc/sprout", "hud"),
    "hud_season_summer": ("lorc/sun", "hud"),
    "hud_season_autumn": ("lorc/falling-leaf", "hud"),
    "hud_season_winter": ("lorc/snowflake-2", "hud"),
    "hud_diplomacy": ("delapouite/shaking-hands", "hud"),
    "hud_chronicle": ("lorc/scroll-unfurled", "hud"),
    "hud_court": ("delapouite/throne-king", "hud"),
    "hud_technologies": ("lorc/gears", "hud"),
    "hud_end_turn": ("lorc/hourglass", "hud"),
    "hud_menu": ("lorc/tied-scroll", "hud"),
    "hud_army": ("delapouite/knight-banner", "hud"),
    "hud_governor": ("lorc/wax-seal", "hud"),
    # --- Branches de compétences (Commandement, Gouvernance, Cour) ---
    "branch_command": ("lorc/crossed-swords", "branch"),
    "branch_governance": ("lorc/scales", "branch"),
    "branch_court": ("delapouite/throne-king", "branch"),
    # --- Catégories de traits ---
    "trait_category_personality": ("lorc/drama-masks", "trait"),
    "trait_category_physical": ("lorc/muscle-up", "trait"),
    "trait_category_martial": ("delapouite/sword-brandish", "trait"),
    "trait_category_governance": ("lorc/wax-seal", "trait"),
    "trait_category_acquired": ("lorc/laurels", "trait"),
}

# data/ directories whose every id must have its own entry (not a fallback).
EXPLICIT_DATA_DIRS: dict[str, str] = {
    "unit_types": "unit",
    "buildings": "building",
    "resources": "resource",
    "technologies": "technology",
    "diets": "resource",
}

# data/ directories covered by a category field: dir -> (field, id prefix).
CATEGORY_DATA_DIRS: dict[str, tuple[str, str]] = {
    "skills": ("branch", "branch_"),
    "traits": ("category", "trait_category_"),
}
