# SS — sol « satellite » de la campagne

Spec : `docs/superpowers/specs/2026-09-30-ss-sol-satellite-design.md` (validée par le joueur le 2026-09-30, autonomie totale).

## État
- Spec commitée. Aucun code.
- **En attente** : le joueur demande d'attendre la fin de la session qui travaille les assets de campagne (`feat/ga3`, worktree `game_project-ga3`) avant de démarrer. Démarrer quand `feat/ga3` est fusionnée dans main (ou que le joueur le dit).

## Prochaine étape
SS1 squelette dans un worktree `feat/ss` : `tools/cent_ans_tools/geo/colormap.py`, `data/map/colormap_style.yaml`, `data/schemas/colormap_style.schema.json`, test pytest désactivé.

## Points ouverts
- Format des tuiles : reprendre celui que charge déjà la pyramide de relief (ADR 0036).
- Numéro d'ADR : prochain libre au moment d'écrire (0139, 0140 pris sur gp-merge).
- Code « SL » déjà pris (sea lanes) → chantier renommé SS.
