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
# UR2: `budget()` only scales named parts, and the `HOODED_PARTS` rider (écorcheurs) uses the
# "Medieval_*" names (`hooded_adventurer.glb`), not "Adventurer_*" ; without this, RIDER_BUDGET
# left them at the full HUMAN_BUDGET (walking figure) size instead of the mounted 0.6 factor,
# which was most of why écorcheurs (cavalry_4) ran well over the LOD0 triangle budget.
RIDER_BUDGET_HOODED = budget(
    **{
        k: 0.57
        for k in (
            "Adventurer_Head",
            "Medieval_Body",
            "Medieval_Legs",
            "Medieval_Feet",
            "Medieval_Head",
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

# --- Lot UR1: regional, faction and period figures ---------------------------------------

WHITE_HARNESS = (0.70, 0.71, 0.74)  # polished 15th-century « harnois blanc »
HOODED_PARTS = [
    (
        "hooded_adventurer.glb",
        ["Medieval_Body", "Medieval_Legs", "Medieval_Feet", "Medieval_Head"],
    ),
    ("adventurer.glb", ["Adventurer_Head"]),
]
HOODED_COLORS = {
    "Medieval_Body:Black": (eq.C_CLOTH, (0.10, 0.08, 0.06)),
    "Medieval_Body:LightBrown": (eq.C_LEATHER, (0.16, 0.09, 0.04)),
    "Medieval_Body:DarkBrown": (eq.C_CLOTH, (0.12, 0.07, 0.03)),
    "Medieval_Body:Metal": (eq.C_MAIL, eq.MAIL),
    "Medieval_Legs:Black": (eq.C_CLOTH, (0.09, 0.07, 0.05)),
    "Medieval_Head:Black": (eq.C_CLOTH, (0.14, 0.10, 0.06)),
    "Medieval_Head:DarkBrown": (eq.C_CLOTH, (0.18, 0.11, 0.05)),
}
PLATE_COLORS = {
    **FIGURES["infantry_0"]["colors"],
}
WHITE_PLATE_COLORS = {
    "King_Body:Metal": (eq.C_PLATE, WHITE_HARNESS),
    "King_Body:Blue": (eq.C_PLATE, WHITE_HARNESS),
    "King_Body:Beige": (eq.C_LEATHER, (0.12, 0.07, 0.035)),
    "King_Body:Metal_Dark": (eq.C_PLATE, (0.55, 0.56, 0.58)),
    "King_Legs:Metal": (eq.C_PLATE, WHITE_HARNESS),
    "King_Legs:DarkBrown": (eq.C_PLATE, (0.55, 0.56, 0.58)),
    "King_Legs:Metal_Dark": (eq.C_PLATE, (0.55, 0.56, 0.58)),
    "King_Feet:Metal": (eq.C_PLATE, WHITE_HARNESS),
}
KING_PARTS = [
    ("king.glb", ["King_Body", "King_Legs", "King_Feet"]),
    ("adventurer.glb", ["Adventurer_Head"]),
]

FIGURES.update(
    {
        # Welsh spearmen: green-and-white levy tunic, bare legs, spear and small buckler;
        # bare head, felt cap or kettle hat.
        "infantry_3": {
            "rig": "human",
            "style": "militia",
            "parts": COMMONER_PARTS,
            "colors": {
                "Green": (eq.C_CLOTH, (0.09, 0.20, 0.07)),
                "LightGreen": (eq.C_CLOTH, (0.72, 0.72, 0.66)),
                "Adventurer_Legs:Brown2": (eq.C_SKIN, (0.50, 0.33, 0.21)),
                "Adventurer_Legs:Brown": (eq.C_SKIN, (0.50, 0.33, 0.21)),
                "Gold": (eq.C_LEATHER, (0.12, 0.08, 0.04)),
            },
            "budget": HUMAN_BUDGET,
            "equipment": [
                ("cloth_cap", 2, {"colour": (0.10, 0.18, 0.07)}),
                ("kettle_hat", 4),
                ("spear", 0, {"length": 2.4}),
                ("round_shield", 0, {"radius": 0.16, "arms": False}),
            ],
            "variants": 3,
        },
        # Scottish schiltron: quilted jack, blue bonnet, kettle hat or bassinet, long
        # spear in both hands, targe slung on the back.
        "infantry_4": {
            "rig": "human",
            "style": "pike",
            "parts": COMMONER_PARTS,
            "colors": {**COMMONER_COLORS, "Green": (eq.C_QUILT, (0.40, 0.35, 0.25))},
            "budget": HUMAN_BUDGET,
            "equipment": [
                ("jack", 0, {"colour": (0.40, 0.34, 0.24), "skirt": 0.26}),
                ("cloth_cap", 1, {"colour": (0.06, 0.08, 0.20)}),
                ("kettle_hat", 2),
                ("bassinet", 4, {"aventail": True}),
                ("pike", 0, {"length": 3.9, "below": 1.1}),
                ("round_shield", 0, {"radius": 0.22, "back": True, "arms": False}),
            ],
            "variants": 3,
        },
        # Flemish goedendag militia: gambeson, livery tabard, kettle hat or bassinet.
        "infantry_5": {
            "rig": "human",
            "style": "militia",
            "parts": COMMONER_PARTS,
            "colors": COMMONER_COLORS,
            "budget": HUMAN_BUDGET,
            "equipment": [
                ("tabard", 0, {"length": 0.3}),
                ("kettle_hat", 1),
                ("bassinet", 2, {"aventail": False}),
                ("goedendag", 0),
            ],
            "variants": 2,
        },
        # Coutiliers: riveted brigandine, sallet (with bevor for one in two), coustille.
        "infantry_6": {
            "rig": "human",
            "style": "militia",
            "parts": COMMONER_PARTS,
            "colors": {**COMMONER_COLORS, "Green": (eq.C_CLOTH, (0.25, 0.20, 0.14))},
            "budget": HUMAN_BUDGET,
            "equipment": [
                ("brigandine", 1, {"colour": (0.30, 0.04, 0.03)}),
                ("brigandine", 2, {"colour": (0.05, 0.08, 0.20)}),
                ("sallet", 1, {"bevor": True}),
                ("sallet", 2),
                ("coustille", 0),
            ],
            "variants": 2,
        },
        # English retinue men-at-arms: plate harness under an armorial jupon, visored
        # bassinet or sallet, pollaxe in both hands.
        "infantry_7": {
            "rig": "human",
            "style": "militia",
            "noble": True,
            "parts": KING_PARTS,
            "colors": PLATE_COLORS,
            "budget": HUMAN_BUDGET,
            "equipment": [
                ("bassinet", 1, {"visor": True}),
                ("sallet", 2, {"bevor": True}),
                ("pollaxe", 0),
            ],
            "variants": 2,
        },
        # Routiers: mismatched gear, hood or bassinet, brigandine or mail, sword and
        # rondache painted with the company's arms.
        "infantry_8": {
            "rig": "human",
            "style": "sword",
            "parts": HOODED_PARTS,
            "colors": HOODED_COLORS,
            "masks": {"Medieval_Head": 1, "Adventurer_Head": 6},
            # UR2: three variants each bake in their own headgear (bassinet, kettle hat, hood)
            # on top of the body ; scaled down a bit more than usual to stay under the 3,000
            # triangle LOD0 budget once every piece of kit is added together.
            "budget": budget(
                Adventurer_Head=0.7, Medieval_Body=0.76, Medieval_Legs=0.78
            ),
            "equipment": [
                ("brigandine", 2, {"colour": (0.20, 0.12, 0.05)}),
                ("bassinet", 2, {"aventail": False}),
                ("kettle_hat", 4),
                ("sword", 0),
                ("round_shield", 0, {"radius": 0.21, "boss": False}),
            ],
            "variants": 3,
        },
        # Francs-archers: livery hoqueton (jack), sallet or felt cap, longbow.
        "archer_3": {
            "rig": "human",
            "style": "bow",
            "parts": COMMONER_PARTS,
            "colors": COMMONER_COLORS,
            "budget": HUMAN_BUDGET,
            "equipment": [
                ("jack", 0, {"colour": (1.0, 1.0, 1.0), "livery": True, "skirt": 0.28}),
                ("sallet", 1),
                ("cloth_cap", 2),
                ("longbow", 0),
                ("quiver", 0),
            ],
            "variants": 2,
        },
        # Gascon crossbowmen: mail shirt, livery tabard, bassinet or kettle hat, pavise.
        "archer_4": {
            "rig": "human",
            "style": "crossbow",
            "parts": COMMONER_PARTS,
            "colors": {**COMMONER_COLORS, "Green": (eq.C_MAIL, eq.MAIL)},
            "budget": HUMAN_BUDGET,
            "equipment": [
                ("tabard", 0, {"length": 0.26}),
                ("bassinet", 1),
                ("kettle_hat", 2),
                ("crossbow", 0),
                ("quiver", 0, {"arrows": False}),
                ("pavise", 0),
            ],
            "variants": 2,
        },
        # Culveriners: jack, sallet or felt cap, hand culverin, powder horn.
        "archer_5": {
            "rig": "human",
            "style": "crossbow",
            "parts": COMMONER_PARTS,
            "colors": COMMONER_COLORS,
            "budget": HUMAN_BUDGET,
            "equipment": [
                ("jack", 0, {"colour": (0.35, 0.30, 0.22), "skirt": 0.22}),
                ("sallet", 1),
                ("cloth_cap", 2, {"colour": (0.30, 0.05, 0.04)}),
                ("hand_culverin", 0),
                ("powder_flask", 0),
            ],
            "variants": 2,
        },
        # Ordonnance gendarmes: white harness, sallet and bevor or visored bassinet,
        # lance with the company pennon; barded horse (chanfron, flanchards), a
        # livery caparison for one in two.
        "cavalry_3": {
            "rig": "cavalry",
            "style": "lance",
            "noble": True,
            "parts": KING_PARTS,
            "colors": WHITE_PLATE_COLORS,
            "budget": RIDER_BUDGET,
            "horse_budget": HORSE_BUDGET,
            "horse_equipment": [
                ("saddle", 0),
                ("chanfron", 0),
                ("flanchards", 1),
                ("caparison", 2),
            ],
            "equipment": [
                ("sallet", 1, {"bevor": True, "colour": WHITE_HARNESS}),
                ("bassinet", 2, {"visor": True, "aventail": False}),
                ("lance", 0),
            ],
            "variants": 2,
        },
        # Écorcheurs: hood or sallet, brigandine, light lance, unarmoured horse.
        "cavalry_4": {
            "rig": "cavalry",
            "style": "lance",
            "parts": HOODED_PARTS,
            "colors": HOODED_COLORS,
            "masks": {"Medieval_Head": 1, "Adventurer_Head": 2},
            "budget": RIDER_BUDGET_HOODED,
            "horse_budget": HORSE_BUDGET,
            "horse_equipment": [("saddle", 0)],
            "equipment": [
                ("brigandine", 2, {"colour": (0.22, 0.05, 0.04), "studs": False}),
                ("sallet", 2),
                ("lance", 0, {"pennon": False}),
            ],
            "variants": 2,
        },
        # Jinetes: light tunic, cap or kettle hat, adarga, javelin; light horse.
        "cavalry_5": {
            "rig": "cavalry",
            "style": "horse_javelin",  # UR2: throws javelins rather than fighting with a lance.
            "parts": COMMONER_PARTS,
            "colors": {
                **COMMONER_COLORS,
                "Green": (eq.C_CLOTH, (0.62, 0.58, 0.50)),
                "LightGreen": (eq.C_LIVERY, (0.9, 0.9, 0.9)),
            },
            "budget": RIDER_BUDGET,
            "horse_budget": HORSE_BUDGET,
            "horse_equipment": [("saddle", 0)],
            "equipment": [
                ("cloth_cap", 1, {"colour": (0.60, 0.56, 0.48)}),
                ("kettle_hat", 2),
                ("adarga", 0),
                ("javelin", 0),
            ],
            "variants": 2,
        },
        # Hobelars: jack, kettle hat or felt cap, light lance, small unarmoured horse.
        "cavalry_6": {
            "rig": "cavalry",
            "style": "lance",
            "parts": COMMONER_PARTS,
            "colors": COMMONER_COLORS,
            "budget": RIDER_BUDGET,
            "horse_budget": HORSE_BUDGET,
            "horse_equipment": [("saddle", 0)],
            "equipment": [
                ("jack", 0, {"colour": (0.45, 0.38, 0.26), "skirt": 0.15}),
                ("kettle_hat", 1),
                ("cloth_cap", 2, {"colour": (0.18, 0.12, 0.06)}),
                ("lance", 0, {"pennon": False}),
            ],
            "variants": 2,
        },
    }
)

# SG3: siege engine crews (servants of the trebuchet, mangonel and bombard, pushers of the
# ram and the siege tower): tunic and hose, cap, straw hat or bare head, no weapon; the
# second figure holds the bombard's rammer. Animated by `SiegeEnginesFx` (clips crank,
# haul, load, swab, push), not by the regiment states.
_CREW_COLORS = {
    "Farmer_Body:Beige": (eq.C_CLOTH, eq.LINEN),
    "Farmer_Body:LightBlue": (eq.C_LIVERY, (0.9, 0.9, 0.9)),
    "Farmer_Pants:LightBlue": (eq.C_CLOTH, HOSE),
    "Farmer_Head:Beige": (eq.C_CLOTH, STRAW),
    "Farmer_Head:Red": (eq.C_CLOTH, (0.25, 0.05, 0.03)),
    "Farmer_Feet:Brown": (eq.C_LEATHER, (0.10, 0.06, 0.03)),
    "Farmer_Feet:Brown2": (eq.C_LEATHER, (0.07, 0.04, 0.02)),
}
_CREW_PARTS = [
    ("farmer.glb", ["Farmer_Body", "Farmer_Pants", "Farmer_Feet", "Farmer_Head"]),
    ("adventurer.glb", ["Adventurer_Head"]),
]
FIGURES.update(
    {
        "crew_0": {
            "rig": "human",
            "style": "crew",
            "parts": _CREW_PARTS,
            "colors": _CREW_COLORS,
            "masks": {"Adventurer_Head": 3, "Farmer_Head": 4},
            "budget": budget(Adventurer_Head=0.75, Farmer_Head=0.8),
            "equipment": [("cloth_cap", 1)],
            "variants": 3,
        },
        "crew_1": {
            "rig": "human",
            "style": "crew",
            "parts": _CREW_PARTS,
            "colors": _CREW_COLORS,
            "masks": {"Adventurer_Head": 1, "Farmer_Head": 2},
            "budget": budget(Adventurer_Head=0.75, Farmer_Head=0.8),
            "equipment": [("cloth_cap", 1), ("rammer", 0)],
            "variants": 2,
        },
    }
)

# Animation style of each figure (read by `BattleSkinned._style`) and noble livery share.
_STYLES = {
    "infantry_0": "sword",
    "infantry_1": "pike",
    "infantry_2": "militia",
    "archer_0": "bow",
    "archer_1": "crossbow",
    "archer_2": "crossbow",
    "cavalry_0": "lance",
    "cavalry_1": "lance",
    "cavalry_2": "horse_bow",
}
for _name, _style in _STYLES.items():
    FIGURES[_name]["style"] = _style
FIGURES["infantry_0"]["noble"] = True
FIGURES["cavalry_0"]["noble"] = True

# --- Lot EP5: standard bearers and musicians ----------------------------------------------
# No weapon nor shield. `pole` = distance along the `Prop` axis from the upper fist to the
# tip of the pole (manifest `pole_top` / `pole_axis`, where Godot hangs the cloth).

FIGURES.update(
    {
        # Standard bearer on foot: harness under an armorial jupon, open bassinet or kettle
        # hat, the pole held in both hands.
        "standard_0": {
            "rig": "human",
            "style": "standard",
            "noble": True,
            "parts": KING_PARTS,
            "colors": PLATE_COLORS,
            "budget": HUMAN_BUDGET,
            "equipment": [
                ("bassinet", 1, {"aventail": False}),
                ("kettle_hat", 2),
                ("standard_pole", 0),
            ],
            "pole": 3.8 - 1.2,
            "variants": 2,
        },
        # Mounted standard bearer: squire in harness on a caparisoned horse, butt of the
        # pole by the right stirrup.
        "standard_1": {
            "rig": "cavalry",
            "style": "standard_mounted",
            "noble": True,
            "parts": KING_PARTS,
            "colors": PLATE_COLORS,
            "budget": RIDER_BUDGET,
            "horse_budget": HORSE_BUDGET,
            "horse_equipment": [("caparison", 0), ("saddle", 0)],
            "equipment": [
                ("bassinet", 1, {"aventail": False}),
                ("bassinet", 2),
                ("standard_pole", 0, {"length": 4.0, "below": 1.1}),
            ],
            "pole": 4.0 - 1.1,
            "variants": 2,
        },
        # Drummer: livery tabard over the tunic, felt cap or kettle hat, tabor and sticks.
        "musician_0": {
            "rig": "human",
            "style": "drum",
            "parts": COMMONER_PARTS,
            "colors": COMMONER_COLORS,
            "budget": HUMAN_BUDGET,
            "equipment": [
                ("tabard", 0),
                ("cloth_cap", 1, {"colour": (0.22, 0.05, 0.04)}),
                ("kettle_hat", 2),
                ("tabor", 0),
                ("drum_sticks", 0),
            ],
            "variants": 2,
        },
        # Trumpeter: livery tabard, felt cap or bare head, long straight busine with a
        # small banner of the arms.
        "musician_1": {
            "rig": "human",
            "style": "horn",
            "parts": COMMONER_PARTS,
            "colors": COMMONER_COLORS,
            "budget": HUMAN_BUDGET,
            "equipment": [
                ("tabard", 0),
                ("cloth_cap", 1, {"colour": (0.08, 0.10, 0.22)}),
                ("busine", 0),
            ],
            "variants": 2,
        },
    }
)

# --- Lot FK2: civilians of the living campaign map ------------------------------------------
# Peasants and townsfolk without armour: undyed or dull cote (no livery: the faction colour
# never shows), hose, felt cap, straw hat or bare head. Named `villager_*` so that they sort
# after every battle figure (`battle_fine_figures.face_of` spreads the faces by sorted name).
# Animated by the `folk` styles of `BattleSkinned.STYLES` (clips scythe, carry, plough).
_FOLK_COLORS = {
    "Farmer_Body:Beige": (eq.C_CLOTH, eq.LINEN),
    "Farmer_Body:LightBlue": (eq.C_CLOTH, (0.17, 0.12, 0.08)),
    "Farmer_Pants:LightBlue": (eq.C_CLOTH, (0.14, 0.12, 0.09)),
    "Farmer_Head:Beige": (eq.C_CLOTH, STRAW),
    "Farmer_Head:Red": (eq.C_CLOTH, (0.16, 0.10, 0.05)),
    "Farmer_Feet:Brown": (eq.C_LEATHER, (0.10, 0.06, 0.03)),
    "Farmer_Feet:Brown2": (eq.C_LEATHER, (0.07, 0.04, 0.02)),
}
_FOLK_BUDGET = budget(Adventurer_Head=0.75, Farmer_Head=0.8)
FIGURES.update(
    {
        # Folk on the roads and in the villages, empty-handed: felt cap (v0), bare head (v1),
        # straw hat (v2), linen coif (v3).
        "villager_0": {
            "rig": "human",
            "style": "folk",
            "parts": _CREW_PARTS,
            "colors": _FOLK_COLORS,
            "masks": {"Adventurer_Head": 0b1011, "Farmer_Head": 0b0100},
            "budget": _FOLK_BUDGET,
            "equipment": [
                ("cloth_cap", 0b0001, {"colour": (0.20, 0.13, 0.07)}),
                ("cloth_cap", 0b1000, {"colour": (0.55, 0.52, 0.45)}),
            ],
            "variants": 4,
        },
        # Mowers (clip scythe): straw hat or bare head, scythe in both hands.
        "villager_1": {
            "rig": "human",
            "style": "folk",
            "parts": _CREW_PARTS,
            "colors": _FOLK_COLORS,
            "masks": {"Adventurer_Head": 0b010, "Farmer_Head": 0b101},
            "budget": _FOLK_BUDGET,
            "equipment": [("scythe", 0)],
            "variants": 3,
        },
        # Angry crowd (revolt): pitchfork (v0), torch (v1, v3), bill (v2); held like the
        # militia's staff weapons (style militia: pike_idle, victory_pike...).
        "villager_2": {
            "rig": "human",
            "style": "militia",
            "parts": _CREW_PARTS,
            "colors": _FOLK_COLORS,
            "masks": {"Adventurer_Head": 0b1011, "Farmer_Head": 0b0100},
            "budget": _FOLK_BUDGET,
            "equipment": [
                ("cloth_cap", 0b0010, {"colour": (0.20, 0.13, 0.07)}),
                ("pitchfork", 0b0001),
                ("torch", 0b1010),
                ("bill", 0b0100),
            ],
            "variants": 4,
        },
        # Carriers (clip carry): sack on the right shoulder; masons, refugees with a bundle.
        "villager_3": {
            "rig": "human",
            "style": "folk_carry",
            "parts": _CREW_PARTS,
            "colors": _FOLK_COLORS,
            "masks": {"Adventurer_Head": 0b011, "Farmer_Head": 0b100},
            "budget": _FOLK_BUDGET,
            "equipment": [
                ("cloth_cap", 0b001, {"colour": (0.55, 0.52, 0.45)}),
                ("sack", 0),
            ],
            "variants": 3,
        },
    }
)
