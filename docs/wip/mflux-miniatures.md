# Images locales gratuites mflux (ADR 0190)

## État
- Modèle `local/z-image-turbo` routé dans `openrouter.request_image` (tests `tools/tests/test_local_art.py`).
- `cent-ans assets illustrations --local` : lot des 148 miniatures de factions en cours (nouveau
  prompt : paysage de la province capitale, scène variée). Blasons inventés, cadre doré fin.

## Lots en cours (10-08)
- A (mech) : FAIT (10-08). `--local` (mflux gratuit, aucun appel réseau ni ligne de budget) sur
  `assets` : illustrations 16:9 ; event-art 16:9 ; codex-art 16:9 ; portraits 3:4 ;
  portrait-archetypes 3:4 ; art-plates 16:9 (loading, ending) / 21:9 (vignette) ;
  entity-icons 1:1 ; ink-icons 1:1 (icônes et médaillons) ; horizon-panoramas 21:9 ;
  materials 1:1 (sans garde de section du grand livre) ; ui-ornaments (aspect du YAML, sans
  `image_size` Gemini, ancre vide ignorée) ; ground-materials generate 1:1 (au lieu de fal.ai).
  Écartées : menu-art et ui-illumination (procéduraux, aucun appel d'image). Tests :
  `tools/tests/test_local_cli.py` ; fixture `fake_mflux` dans `tests/conftest.py`.
- B (dev) : chaîne figurines GA3 (`tools/experiments/ga3_fal_*.py`) : planche de référence
  locale (Z-Image img2img) + détourage local (rembg) ; seul TRELLIS reste payant (0,02 $).

## Ensuite
- Rogner le cadre doré des miniatures de factions, contrôle visuel d'un échantillon, commit.
- Incrustation des vrais blasons (données `heraldry`).
- Essais réels de A et B quand le lot de factions est fini (GPU libre).
