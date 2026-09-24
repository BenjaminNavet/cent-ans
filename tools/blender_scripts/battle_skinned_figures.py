"""Figure recipes of the skinned battle figures (lot V2).

A recipe names the Quaternius parts to assemble (file, object names), recolours their
materials (`colors`: Quaternius material -> (code, linear colour)), sets the triangle budget
of each part per level of detail (`budget`: one dict per level, `*` = default) and lists the
equipment builders of `battle_skinned_equipment` (name, variant mask, keyword arguments).
Variant masks: bit v set = shown for soldiers of variant v; 0 = always shown.
"""

import battle_skinned_equipment as eq

HUMAN_BUDGET = [
    {"*": 450, "King_Body": 750, "Adventurer_Body": 650, "Medieval_Body": 650, "Farmer_Body": 650, "Adventurer_Head": 650, "Farmer_Head": 450},
    {"*": 90, "King_Body": 150, "Adventurer_Body": 140, "Medieval_Body": 140, "Farmer_Body": 140, "Adventurer_Head": 110, "Farmer_Head": 90},
    {"*": 35, "King_Body": 60, "Adventurer_Body": 55, "Medieval_Body": 55, "Farmer_Body": 55, "Adventurer_Head": 40, "Farmer_Head": 35},
]
HUMAN_BUDGET[0].update({"King_Legs": 380, "King_Feet": 120, "Adventurer_Legs": 300, "Adventurer_Feet": 100})
HUMAN_BUDGET[1].update({"King_Legs": 80, "King_Feet": 24, "Adventurer_Legs": 70, "Adventurer_Feet": 20})
HUMAN_BUDGET[2].update({"King_Legs": 30, "King_Feet": 10, "Adventurer_Legs": 28, "Adventurer_Feet": 8})

FIGURES = {
    # Man-at-arms on foot: mail, coat of plates under a livery jupon, bassinet and aventail.
    "infantry_0": {
        "rig": "human",
        "parts": [
            ("king.glb", ["King_Body", "King_Legs", "King_Feet"]),
            ("adventurer.glb", ["Adventurer_Head"]),
        ],
        "colors": {
            "Metal": (eq.C_LIVERY, (0.8, 0.8, 0.8)),
            "Blue": (eq.C_MAIL, eq.MAIL),
            "Beige": (eq.C_CLOTH, eq.LINEN),
        },
        "budget": HUMAN_BUDGET,
        "equipment": [("bassinet", 0), ("sword", 0)],
        "variants": 1,
    },
}
