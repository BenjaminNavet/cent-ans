# OM-C1 — complétude des données annexes (nouvelles provinces et factions)

Branche `feat/om-c1` (depuis `feat/om`). État : TERMINÉ (données ; pas de cargo, Godot ni géo).

## Recensement (fichier → état)
| Fichier | État |
|---|---|
| `data/audio/music.json` (`culture_regions`) | fait : 24 cultures orientales rattachées à une région musicale existante (iberia : arabe, maghrébin, berbère ; italy : grec, turc, arménien, géorgien, balkaniques… ; burgundy : polonais, hongrois, lituanien, baltes) ; russe, steppe, finnois, norvégien : repli `campaign` |
| `data/rules/battle_decor.json` | fait : profil `orient` (provinces musulmanes plaines/collines, sans vignes), `midi` étendu (Grèce, Égée, Chypre, Croatie, Crimée chrétienne) ; steppe/désert déjà par terrain |
| `data/speeches/battle_speeches.json` | fait : lignes `steppe` et `desert` ; ouvertures/conclusions par faction : repli `default` |
| `data/codex/` | fait : 17 fiches de grandes puissances nouvelles (les factions d'origine n'en ont que 9) |
| `data/portraits/archetypes.json`, `data/ui/front_end.json`, `data/rules/mercenaries.json` | déjà complets |
| `data/ai/doctrines.json` | repli `default` (11 factions d'origine seulement ; pas d'unités orientales, hors périmètre v1) |
| unités par culture (`required_culture`) | repli (types existants), hors périmètre v1 |
| encounters, events | repli (génériques, `province: all`) ; contenu oriental hors périmètre v1 |
| `data/fx/horizon.json`, `data/map/*` | géo, non régénéré (hors mandat) |
| écus `game/assets/heraldry/fac_*.png` | manque 74 sur 86 (asset binaire, hors mandat) |

## Test
`tools/tests/test_battle_speeches_schema.py` : liste `BATTLE_TERRAINS` complétée (steppe, desert).
