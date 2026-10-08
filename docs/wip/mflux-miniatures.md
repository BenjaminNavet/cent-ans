# Images locales gratuites mflux (ADR 0190)

## État
- Modèle `local/z-image-turbo` routé dans `openrouter.request_image` (tests `tools/tests/test_local_art.py`).
- `cent-ans assets illustrations --local` : lot des 148 miniatures de factions en cours (nouveau
  prompt : paysage de la province capitale, scène variée). Blasons inventés, cadre doré fin.

## Lots en cours (10-08)
- A (mech) : `--local` sur toutes les commandes d'images 2D de `cli.py` (événements, portraits,
  codex, planches, icônes, panoramas, matériaux, sols, ornements), format par commande.
- B (dev) : chaîne figurines GA3 (`tools/experiments/ga3_fal_*.py`) : planche de référence
  locale (Z-Image img2img) + détourage local (rembg) ; seul TRELLIS reste payant (0,02 $).

## Ensuite
- Rogner le cadre doré des miniatures de factions, contrôle visuel d'un échantillon, commit.
- Incrustation des vrais blasons (données `heraldry`).
- Essais réels de A et B quand le lot de factions est fini (GPU libre).
