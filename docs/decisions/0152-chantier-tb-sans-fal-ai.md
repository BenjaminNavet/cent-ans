# 0152 — Chantier TB sans fal.ai

Date : 2026-10-02. Statut : acceptée (décision du joueur le 02/10). Remplace l'ADR 0149.
Plan : `docs/design/2026-10-02-campagne-tob.md`.

## Contexte
L'ADR 0149 ouvrait une enveloppe fal.ai de 25 $ hors plafond pour le chantier TB. À la reprise
locale du 02/10, le compte fal.ai était verrouillé (« Exhausted balance ») avant tout appel. Le
joueur n'a plus de crédit et a décidé de ne pas recharger : « on laisse tomber ça ».

## Décision
- Le chantier TB se fait **sans fal.ai** et sans autre service payant : dépense TB = 0 $.
  L'enveloppe de l'ADR 0149 est annulée ; aucun appel n'a été facturé.
- **TB0** : pas de repeints. Les critères de réussite sont les références Thrones of Britannia
  rapatriées en local (`~/.cache/cent_ans/tb/refs/`, non versionnées) et nos captures « avant »
  (`~/.cache/cent_ans/tb/ours/`). Le joueur juge la direction sur le résultat de la vague 1
  (TB1, TB2) avant TB3.
- **TB3** (bâtiments hors les murs à 3 niveaux) : maquettes tirées des kits existants (GA3,
  ADR 0138, villages TF) et de Blender procédural, au lieu de générations.
- **TB5** (falaises, plages) : matières tirées des textures HB déjà dans le dépôt ou procédurales
  dans le shader.
- **TB7** (ornements d'interface) : reporté ; à refaire avec l'existant si le joueur le demande.

## Conséquences
- Le script `tools/experiments/tb0_target_board.py` et l'exception `docs/img/tb/` du `.gitignore`
  sont retirés.
- La section TB de `docs/budget.md` reste vide et le dit.
- TB3 demande plus de travail Blender et moins de variété que prévu ; son périmètre sera recadré
  au lancement de la vague 2.
