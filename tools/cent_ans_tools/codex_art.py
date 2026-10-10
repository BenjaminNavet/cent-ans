"""Codex miniatures (places, battles, daily life, institutions...) through OpenRouter.

A Codex entry whose ``entity`` already has an image (event miniature, encyclopedia
illustration or portrait) reuses it in game; only the other entries get their own
miniature, saved as ``game/assets/illustrations/<cdx id>.jpg`` (640x360 JPEG, same crop
as the encyclopedia illustrations). Prompts are built from the entry data only (title,
category, period, summary without the ``[[links]]`` markup).

The paid batch reuses :func:`cent_ans_tools.portraits.generate`.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

from cent_ans_tools import entry_art
from cent_ans_tools.portraits import DATA_DIR, REPO_DIR, PortraitJob

GAME_ASSETS_DIR = REPO_DIR / "game" / "assets"
# Generation order: most visual families first, so a short budget covers them.
# Characters are left out (portraits/chr_<slug>.png).
CATEGORIES = (
    "animal",
    "oiseau",
    "arbre",
    "roche",
    "plante",
    "lieu",
    "bataille",
    "guerre",
    "evenement",
    "vie_quotidienne",
    "societe",
    "economie",
    "institution",
    "religion",
    "savoir",
    "medecine",
    "dynastie",
    "cuisine",
    "recette",
    "ingredient",
)
# Nature entries get a bestiary / herbal plate instead of a scene (isolated subject on vellum).
NATURE_CATEGORIES = ("animal", "oiseau", "arbre", "roche", "plante")
NATURE_STYLE = (
    "Style: plate from a 14th-century illuminated medieval {book}, painted on cream "
    "vellum parchment. A single isolated subject, centred in the middle of the image "
    "and filling about 60 % of its height, with a plain pale vellum background and "
    "generous empty margins left and right. Egg tempera colours, fine brown-black ink "
    "outlines, delicate gold leaf touches, naturalistic yet slightly stylised "
    "Gothic manner. No text, no letters, no captions, no frame, no border, no "
    "modern elements."
)
NATURE_BOOKS = {
    "animal": ("bestiary", "A bestiary plate showing {subject}."),
    "oiseau": ("bestiary", "A bestiary plate showing {subject}."),
    "arbre": ("herbal", "A herbal plate showing {subject}."),
    "plante": ("herbal", "A herbal plate showing {subject}."),
    "roche": (
        "book of stones (lapidary)",
        "A lapidary plate showing {subject}, drawn as a landscape detail, no figures.",
    ),
}
# English subject per entry, so that the model draws the right species (Z-Image reads
# French titles poorly). Entries absent here fall back to the French title and Latin name.
NATURE_SUBJECTS = {
    "cdx_aiguilles_alpines": "jagged snow-capped Alpine needle peaks above a mountain pass",
    "cdx_ane_et_mulet": "a donkey and a mule side by side, with pack saddles",
    "cdx_arbousier": "an Arbutus unedo shrub branch with glossy evergreen lanceolate leaves, drooping clusters of small white urn-shaped flowers and round rough-skinned orange-red berries (not a strawberry plant)",
    "cdx_aurochs": "an aurochs, huge wild black bull with long forward-curving horns",
    "cdx_bison": "a European bison with shaggy hump and short horns",
    "cdx_blocs_erratiques": "huge isolated erratic boulders lying in a meadow",
    "cdx_bouleau": "a whole silver birch tree on plain parchment, slender white trunk with black marks and drooping thin branches with small triangular leaves",
    "cdx_bouquetin": "an alpine ibex with large ridged curved horns on a rock",
    "cdx_bovins": "a medieval ox and a cow",
    "cdx_castor": "a European beaver gnawing a branch",
    "cdx_cerf": "a red deer stag (Cervus elaphus) with natural brown fur and large antlers, and a hind",
    "cdx_chameaux": "a dromedary and a two-humped Bactrian camel",
    "cdx_chamois": "a chamois with small hooked horns on a rock",
    "cdx_chaos_granitiques": "a chaos of rounded granite boulders heaped on a hillside",
    "cdx_chataignier": "a sweet chestnut tree branch with spiny husks and chestnuts",
    "cdx_chene": "a large oak tree with lobed leaves and acorns",
    "cdx_chene_kermes": "a low kermes oak shrub with small spiny holly-like leaves and acorns",
    "cdx_chene_vert": "a holm oak, evergreen tree with dark glossy leathery leaves and acorns",
    "cdx_cheval_de_trait": "a heavy draught horse beside a small white Camargue horse",
    "cdx_chevre": "a domestic goat with curved horns and a beard",
    "cdx_chevreuil": "a roe deer buck with short antlers",
    "cdx_cigogne": "a white stork standing at its large stick nest",
    "cdx_corneille": "a carrion crow and a raven, black birds",
    "cdx_cretes_stratifiees": "tilted stratified limestone ridges with layered rock bands",
    "cdx_cypres": "a tall dark slender Mediterranean cypress tree",
    "cdx_elan": "a moose with broad palmate antlers",
    "cdx_epicea": "a Norway spruce tree with drooping branches and a hanging cone",
    "cdx_erable": "a maple tree branch with palmate leaves and winged seeds",
    "cdx_etourneau": "a common starling, dark speckled glossy bird",
    "cdx_falaises_calcaires": "tall white limestone sea cliffs",
    "cdx_flamant_rose": "a pink flamingo standing on one leg in shallow water",
    "cdx_frene": "an ash tree branch with pinnate leaves and winged keys",
    "cdx_goeland": "a seagull and a gull in flight and at rest",
    "cdx_gres_rouge": "red sandstone rock formation, layered rusty red cliffs",
    "cdx_grue": "a common crane, tall grey bird with long neck and black head",
    "cdx_haies_et_buissons": "a hedge of hawthorn, blackthorn, hazel and dog rose with berries and blossoms",
    "cdx_hetre": "a beech tree branch with oval leaves and beechnuts",
    "cdx_lentisque": "a mastic shrub (Pistacia lentiscus) branch with compound leaves and red berries",
    "cdx_lievre": "a brown hare sitting upright with long ears",
    "cdx_megalithes": "standing stones and a dolmen, neolithic megaliths in a field",
    "cdx_meleze": "a European larch branch with soft needle tufts and small cones",
    "cdx_morse": "a walrus with long tusks on an ice floe",
    "cdx_mouton": "a medieval domestic sheep with thick wool, and a ram with curled horns",
    "cdx_oie": "a domestic goose and a wild grey goose",
    "cdx_olivier": "an olive tree branch with narrow silver-green leaves and olives",
    "cdx_ours": "a brown bear standing on all fours",
    "cdx_peuplier": "a tall poplar tree with trembling leaves",
    "cdx_phoque": "a grey harbour seal, a marine mammal with a smooth round head, whiskers and flippers, lying on a rock by the sea",
    "cdx_pin_d_alep": "an Aleppo pine, twisted trunk with sparse crown and cones",
    "cdx_pin_maritime": "a maritime pine, tall straight trunk with a cone",
    "cdx_pin_noir": "a black pine, dark bark with a dense crown and cones",
    "cdx_pin_parasol": "a stone pine, umbrella-shaped crown with a large cone and pine nuts",
    "cdx_pin_sylvestre": "a Scots pine with orange upper bark, blue-green needles and cones",
    "cdx_pommier": "an apple tree branch with blossom and red apples",
    "cdx_pavot": "an opium poppy plant with a flower and a round seed capsule",
    "cdx_saule": "a white willow branch with long narrow leaves and catkins, a strip of bark",
    "cdx_porc": "a medieval domestic pig and a piglet",
    "cdx_rapaces_et_fauconnerie": "a falcon perched on a gloved fist with jesses and hood, hand only",
    "cdx_renard": "a red fox with bushy tail",
    "cdx_renne": "a reindeer with branching antlers",
    "cdx_roches_volcaniques": "black volcanic rock with a basalt column outcrop and a small volcanic cone",
    "cdx_sanglier": "a wild boar with tusks and bristled back",
    "cdx_sapin": "a silver fir tree with upright cones and flat needles",
    "cdx_tarpan": "a tarpan, small wild steppe horse, mouse-dun coat with dark dorsal stripe and upright mane",
    "cdx_taureau_de_camargue": "a black Camargue bull with lyre-shaped horns",
    "cdx_terres_arides": "an arid stony plateau with gypsum hills and sparse dry shrubs",
}
# Economy entries: one concrete scene per resource (generic « procession » scenes and painted
# titles came out of the title-based prompt). The title is left out of the prompt on purpose.
ECONOMY_SCENES = {
    "cdx_ble": "peasants reaping golden wheat with sickles and binding sheaves in a field, a village behind",
    "cdx_bois": "woodcutters felling oaks with axes and loading logs on an ox cart in a forest",
    "cdx_drap": "weavers at a wide wooden loom and dyers in a cloth workshop, bolts of coloured woollen cloth",
    "cdx_fer": "blacksmiths at a forge hammering red-hot iron on an anvil, bellows and iron bars",
    "cdx_laine": "shepherds shearing sheep and merchants loading sacks of wool on a cart",
    "cdx_pierre": "stonemasons cutting and carving blocks in a quarry, a treadwheel crane lifting a stone",
    "cdx_poisson": "fishermen hauling nets from a boat, baskets of fish and barrels on a quay",
    "cdx_vin": "peasants treading grapes in a vat and a wine press with barrels, in a vineyard",
}
# Where an entity image may already live, relative to game/assets.
ENTITY_IMAGES = ("events/{}.jpg", "illustrations/{}.jpg", "portraits/{}.png")
# Game fallback by id (codex_window.gd SLUG_ART): fiche cdx_x of the portrait chr_x.
# icons/entity is deliberately not counted: a small icon is not a miniature.
SLUG_IMAGES = ("portraits/chr_{}.png",)

_LINK = re.compile(r"\[\[[^\]|]+\|([^\]]+)\]\]|\[\[([^\]]+)\]\]")

convert = entry_art.convert


def plain_text(text: str) -> str:
    """Codex text without the ``[[id|label]]`` link markup (label kept)."""
    return _LINK.sub(lambda match: match.group(1) or match.group(2), text)


def has_entity_image(entry: dict, assets_dir: Path = GAME_ASSETS_DIR) -> bool:
    """True when the entry's game entity already has an image the Codex can reuse."""
    entity = entry.get("entity", "")
    slug = str(entry.get("id", "")).removeprefix("cdx_")
    by_entity = bool(entity) and any(
        (assets_dir / pattern.format(entity)).exists() for pattern in ENTITY_IMAGES
    )
    by_slug = bool(slug) and any(
        (assets_dir / pattern.format(slug)).exists() for pattern in SLUG_IMAGES
    )
    return by_entity or by_slug


def build_prompt(entry: dict) -> str:
    """Miniature prompt for one Codex entry, from its data only.

    Nature entries (:data:`NATURE_CATEGORIES`) get a bestiary / herbal plate.
    """
    if entry["category"] in NATURE_CATEGORIES:
        book, template = NATURE_BOOKS[entry["category"]]
        latin = f" ({entry['latin']})" if entry.get("latin") else ""
        subject = NATURE_SUBJECTS.get(entry["id"]) or f"« {entry['title']} »{latin}"
        return "\n".join(
            [template.format(subject=subject), NATURE_STYLE.format(book=book)]
        )
    if entry["id"] in ECONOMY_SCENES:
        return "\n".join(
            [
                f"A medieval manuscript miniature showing {ECONOMY_SCENES[entry['id']]}.",
                entry_art.STYLE,
            ]
        )
    era = entry.get("era") or {}
    start, end = str(era.get("from", "")), str(era.get("to", ""))
    period = start if start == end or not end else f"{start}-{end}"
    lines = [
        f"A scene illustrating « {entry['title']} » (French encyclopedia topic, "
        f"category {entry['category'].replace('_', ' ')}"
        + (f", period {period}" if period else "")
        + "), in the world of the Hundred Years' War.",
        f"Subject: {plain_text(entry.get('summary', ''))}",
        entry_art.STYLE,
    ]
    return "\n".join(lines)


def plan(
    data_dir: Path = DATA_DIR,
    assets_dir: Path = GAME_ASSETS_DIR,
    categories: tuple[str, ...] = CATEGORIES,
    limit: int | None = None,
) -> list[PortraitJob]:
    """Missing Codex miniatures, by category priority then id, up to ``limit``."""
    entries = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in sorted((data_dir / "codex").glob("cdx_*.json"))
    ]
    jobs = []
    for category in categories:
        for entry in entries:
            # Nature plates always get their own plate: a tech or event scene is off-subject.
            reuse = category not in NATURE_CATEGORIES and has_entity_image(
                entry, assets_dir
            )
            if entry["category"] != category or reuse:
                continue
            out_path = assets_dir / "illustrations" / f"{entry['id']}.jpg"
            if out_path.exists():
                continue
            jobs.append(PortraitJob(entry["id"], build_prompt(entry), out_path))
            if limit is not None and len(jobs) >= limit:
                return jobs
    return jobs
