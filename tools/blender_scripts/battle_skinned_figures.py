"""Figure recipes of the skinned battle figures (lot V2).

A recipe names the Quaternius parts to assemble (file, object names), recolours their
materials (`colors`: "Part:Material" or "Material" -> (code, linear colour)), sets the
triangle budget of each part per level of detail (`budget`: one dict per level, `*` =
default) and lists the equipment builders of `battle_skinned_weapons` /
`battle_skinned_equipment` (name, variant mask, keyword arguments).
Variant masks: bit v set = shown for soldiers of variant v; 0 = always shown (`masks` does
the same for whole Quaternius parts, e.g. two heads).
"""

import copy

import battle_skinned_equipment as eq

# Variant mask of the Genoese back pavise: shown for variants 0 and 1, plus bit 7 as a
# part flag read by battle_soldier_skinned.gdshader (`hide_pavise`, BV3).
PAVISE_MASK = 0b1000_0011

HUMAN_BUDGET = [
    {
        "*": 450,
        "King_Body": 750,
        "King_Legs": 380,
        "King_Feet": 120,
        "Adventurer_Body": 650,
        "Adventurer_Legs": 300,
        "Adventurer_Feet": 100,
        "Adventurer_Head": 650,
        "Farmer_Body": 650,
        "Farmer_Pants": 300,
        "Farmer_Feet": 100,
        "Farmer_Head": 420,
    },
    {
        "*": 90,
        "King_Body": 150,
        "King_Legs": 80,
        "King_Feet": 24,
        "Adventurer_Body": 140,
        "Adventurer_Legs": 70,
        "Adventurer_Feet": 20,
        "Adventurer_Head": 110,
        "Farmer_Body": 140,
        "Farmer_Pants": 70,
        "Farmer_Feet": 20,
        "Farmer_Head": 80,
    },
    {
        "*": 35,
        "King_Body": 60,
        "King_Legs": 30,
        "King_Feet": 10,
        "Adventurer_Body": 55,
        "Adventurer_Legs": 28,
        "Adventurer_Feet": 8,
        "Adventurer_Head": 40,
        "Farmer_Body": 55,
        "Farmer_Pants": 28,
        "Farmer_Feet": 8,
        "Farmer_Head": 30,
    },
]


def budget(**parts):
    """Return a copy of HUMAN_BUDGET with some parts scaled, e.g. `Adventurer_Head=0.7`."""
    out = copy.deepcopy(HUMAN_BUDGET)
    for level in out:
        for name, factor in parts.items():
            level[name] = int(level.get(name, level["*"]) * factor)
    return out


GAMBESON = (0.50, 0.44, 0.32)
HOSE = (0.12, 0.08, 0.05)
RUSSET = (0.22, 0.10, 0.05)
STRAW = (0.45, 0.34, 0.15)

# Commoners: tunic and hose (Adventurer), front panel in livery, quilted gambeson sides.
COMMONER_COLORS = {
    "Green": (eq.C_QUILT, GAMBESON),
    "LightGreen": (eq.C_LIVERY, (0.9, 0.9, 0.9)),
    "Adventurer_Legs:Brown2": (eq.C_CLOTH, HOSE),
    "Adventurer_Legs:Brown": (eq.C_LEATHER, (0.10, 0.06, 0.03)),
    "Gold": (eq.C_LEATHER, (0.12, 0.08, 0.04)),
}
RIDER_BUDGET = budget(
    **{
        k: 0.6
        for k in (
            "King_Body",
            "King_Legs",
            "King_Feet",
            "Adventurer_Body",
            "Adventurer_Legs",
            "Adventurer_Feet",
            "Adventurer_Head",
        )
    }
)
HORSE_BUDGET = [1100, 280, 110]

COMMONER_PARTS = [
    (
        "adventurer.glb",
        ["Adventurer_Body", "Adventurer_Legs", "Adventurer_Feet", "Adventurer_Head"],
    )
]

FIGURES = {
    # Man-at-arms on foot: mail, coat of plates under a livery jupon, bassinet and aventail,
    # arming sword; half of them with a heater shield painted with the arms.
    "infantry_0": {
        "rig": "human",
        "parts": [
            ("king.glb", ["King_Body", "King_Legs", "King_Feet"]),
            ("adventurer.glb", ["Adventurer_Head"]),
        ],
        "colors": {
            "King_Body:Metal": (eq.C_LIVERY, (0.85, 0.85, 0.85)),
            "King_Body:Blue": (eq.C_MAIL, eq.MAIL),
            "King_Body:Beige": (eq.C_LEATHER, (0.12, 0.07, 0.035)),
            "King_Body:Metal_Dark": (eq.C_PLATE, (0.42, 0.43, 0.45)),
            "King_Legs:Metal": (eq.C_PLATE, eq.STEEL),
            "King_Legs:DarkBrown": (eq.C_MAIL, eq.MAIL),
            "King_Legs:Metal_Dark": (eq.C_PLATE, (0.42, 0.43, 0.45)),
            "King_Feet:Metal": (eq.C_PLATE, (0.42, 0.43, 0.45)),
        },
        "budget": HUMAN_BUDGET,
        "equipment": [("bassinet", 0), ("sword", 0), ("heater_shield", 1)],
        "variants": 2,
    },
    # Flemish pikemen: gambeson, kettle hat or open bassinet, long pike.
    "infantry_1": {
        "rig": "human",
        "parts": COMMONER_PARTS,
        "colors": COMMONER_COLORS,
        "budget": HUMAN_BUDGET,
        "equipment": [
            ("kettle_hat", 1),
            ("bassinet", 2, {"aventail": False}),
            ("cloth_cap", 4),
            ("pike", 0),
        ],
        "variants": 3,
    },
    # Urban militia and peasants: tunic and livery tabard, cap or kettle hat, spear or bill;
    # a quarter of plain peasants (straw hat, pitchfork).
    "infantry_2": {
        "rig": "human",
        "parts": [
            (
                "farmer.glb",
                ["Farmer_Body", "Farmer_Pants", "Farmer_Feet", "Farmer_Head"],
            ),
            ("adventurer.glb", ["Adventurer_Head"]),
        ],
        "colors": {
            "Farmer_Body:Beige": (eq.C_CLOTH, eq.LINEN),
            "Farmer_Body:LightBlue": (eq.C_CLOTH, RUSSET),
            "Farmer_Pants:LightBlue": (eq.C_CLOTH, HOSE),
            "Farmer_Head:Beige": (eq.C_CLOTH, STRAW),
            "Farmer_Head:Red": (eq.C_CLOTH, (0.25, 0.05, 0.03)),
            "Farmer_Feet:Brown": (eq.C_LEATHER, (0.10, 0.06, 0.03)),
            "Farmer_Feet:Brown2": (eq.C_LEATHER, (0.07, 0.04, 0.02)),
        },
        "masks": {"Adventurer_Head": 7, "Farmer_Head": 8},
        "budget": budget(Adventurer_Head=0.75, Farmer_Head=0.8),
        "equipment": [
            ("tabard", 3),
            ("cloth_cap", 1),
            ("kettle_hat", 2),
            ("spear", 5),
            ("bill", 2),
            ("pitchfork", 8),
        ],
        "variants": 4,
    },
    # English longbowmen: livery jacket, kettle hat, felt cap or bare head, yew longbow,
    # arrow bag.
    "archer_0": {
        "rig": "human",
        "parts": COMMONER_PARTS,
        "colors": {**COMMONER_COLORS, "Green": (eq.C_CLOTH, (0.30, 0.25, 0.16))},
        "budget": HUMAN_BUDGET,
        "equipment": [
            ("kettle_hat", 1),
            ("cloth_cap", 2),
            ("longbow", 0),
            ("quiver", 0),
        ],
        "variants": 3,
    },
    # Crossbowmen: gambeson, kettle hat or bassinet, crossbow, bolt case.
    "archer_1": {
        "rig": "human",
        "parts": COMMONER_PARTS,
        "colors": COMMONER_COLORS,
        "budget": HUMAN_BUDGET,
        "equipment": [
            ("kettle_hat", 1),
            ("bassinet", 2, {"aventail": False}),
            ("crossbow", 0),
            ("quiver", 0, {"arrows": False}),
        ],
        "variants": 2,
    },
    # Knights: as the men-at-arms, great helm or bassinet, shield and lance, horse in a
    # caparison of the side's livery and arms.
    "cavalry_0": {
        "rig": "cavalry",
        "parts": [
            ("king.glb", ["King_Body", "King_Legs", "King_Feet"]),
            ("adventurer.glb", ["Adventurer_Head"]),
        ],
        "colors": {},  # filled below from infantry_0
        "budget": RIDER_BUDGET,
        "horse_budget": HORSE_BUDGET,
        "horse_equipment": [("caparison", 0), ("saddle", 0)],
        "equipment": [
            ("great_helm", 1),
            ("bassinet", 2),
            ("heater_shield", 0),
            ("lance", 0),
        ],
        "variants": 2,
    },
    # Mounted sergeants: gambeson and mail, kettle hat or bassinet, lance without pennon.
    "cavalry_1": {
        "rig": "cavalry",
        "parts": COMMONER_PARTS,
        "colors": COMMONER_COLORS,
        "budget": RIDER_BUDGET,
        "horse_budget": HORSE_BUDGET,
        "horse_equipment": [("saddle", 0)],
        "equipment": [
            ("kettle_hat", 1),
            ("bassinet", 2),
            ("lance", 0, {"pennon": False}),
        ],
        "variants": 2,
    },
    # Mounted archers: archer's kit on a riding horse.
    "cavalry_2": {
        "rig": "cavalry",
        "parts": COMMONER_PARTS,
        "colors": {**COMMONER_COLORS, "Green": (eq.C_CLOTH, (0.30, 0.25, 0.16))},
        "budget": RIDER_BUDGET,
        "horse_budget": HORSE_BUDGET,
        "horse_equipment": [("saddle", 0)],
        "equipment": [
            ("kettle_hat", 1),
            ("cloth_cap", 2),
            ("longbow", 0),
            ("quiver", 0),
        ],
        "variants": 2,
    },
    # Genoese crossbowmen: as above with bassinet and aventail, pavise on the back.
    "archer_2": {
        "rig": "human",
        "parts": COMMONER_PARTS,
        "colors": {**COMMONER_COLORS, "Green": (eq.C_QUILT, (0.55, 0.52, 0.45))},
        "budget": HUMAN_BUDGET,
        "equipment": [
            ("bassinet", 1),
            ("kettle_hat", 2),
            ("crossbow", 0),
            ("quiver", 0, {"arrows": False}),
            # Bit 7 flags the back pavise (hidden by the shader once the row is planted, BV3).
            ("pavise", PAVISE_MASK),
        ],
        "variants": 2,
    },
}

FIGURES["cavalry_0"]["colors"] = FIGURES["infantry_0"]["colors"]
