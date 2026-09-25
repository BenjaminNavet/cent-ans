# EP4 — son de mêlée de proximité

Lot du chantier « batailles épiques » (`docs/wip/epic.md`). Objectif : bataille qui sonne
massive — cris et fracas d'armes en zoomant près d'un combat, émetteurs par front de mêlée,
couches selon la distance caméra. S'appuie sur AU1 (`docs/wip/au1-audio.md`,
`game/scripts/audio/battle_audio.gd`) ; ne touche pas `battle_voices.gd` ni `battle_speech.gd`.

## État

- [x] Sources CC0 repérées et vérifiées page par page (licence CC0 1.0 uniquement) : 27 nouveaux
      sons Freesound (acier/acier, acier/bois, armure/maille, effort/rage, râles, chutes de
      corps/armure, chevaux, foule de mêlée). Détail : `game/assets/audio/SOURCE.md`.
- [ ] `tools/cent_ans_tools/audio_bank.py` : nouveaux `Clip` (comptes cibles : 12 acier/acier,
      8 acier/bois, 6 armure/maille, 10 effort/rage, 10 râles, 6 chutes, 6 hennissements,
      3 nappes de mêlée massives) + génération des fichiers `.ogg`.
- [ ] `data/audio/sound_bank.json` + schéma : nouveaux événements, section `fronts` (paramètres
      des émetteurs par front : nombre max, portées, densité).
- [ ] `battle_audio.gd` : regroupement des régiments en mêlée par front (paires `id`/`target`),
      émetteurs 3D les plus proches de la caméra, densité/volume selon effectif engagé et pertes
      récentes, couches proche/moyen/loin, sélection sans répétition, événements charge de
      cavalerie / déroute qui se déplace / mort du général / volée au-dessus de la caméra.
- [ ] Test headless `game/tests/ep4_audio_test.gd`.
- [ ] Smoke, ruff, merge main, rapport.

## Décisions

- Ordres criés génériques (spec) : aucune source CC0 de cris de commandement sans langue
  identifiable trouvée (résultats en anglais/allemand explicites, ou hors-sujet). Non ajouté ;
  noté comme limite. Les cris de charge/guerre existants (AU1) couvrent déjà les ordres non
  verbaux à l'engagement.
- Choc de charge de cavalerie : pas de source CC0 dédiée trouvée (« horse charge impact »,
  « cavalry charge » infructueux) ; construit à partir de sources déjà vérifiées (galop
  `cavalry_bed`, chocs boucliers/épées) plutôt que d'un enregistrement dédié.

## Prochaine étape

Écrire les `Clip` dans `audio_bank.py`, lancer la génération, puis le regroupement par front dans
`battle_audio.gd`.
