# Lot TEXTES (boucle juge 2, itération 1)

Branche `worktree-agent-acc0be412a5869b78`, base main 6a601816a. Rapports : `out/juge2/iter-01-histux.md`, `iter-01-design.md`.

## Fait
1. `out/camera.txt` n'est lu qu'avec la feature `dev` (`client/src/campaign/camera/mod.rs`) ; `CENT_ANS_CAMERA` reste actif partout.
2. `data/ui/hud_labels.json` : plus d'abréviations (Hommes, Prestige, Mécontentement, y compris barre étroite) ; « Désordre » remplacé par « Mécontentement » (causes, fiche province). Infobulles des boutons : « Raccourci : F4 » sur une ligne à part ; lettre du raccourci en corps 12.
3. « colonie(s) » -> « localité(s) » (data/ui, codex, bâtiments, tutoriel, vocations, textes Rust de client/ et core/). Exceptions volontaires : colonies d'animaux (phoque, flamant, corneille), colonies norroises, colonies génoises/vénitiennes (provinces). Alerte : « 32 localités sans chantier : le trésor (28 000 livres) permet d'y bâtir ».
4. Codex : les entrées dont l'époque commence après l'année de la campagne restent masquées tant qu'elles ne sont pas découvertes (`CodexStore::set_year`, `is_future`).
5. Conseiller par faction : `speakers` / `default_speaker` dans `data/voice/advisor.json` (France : Pierre Roger, archevêque de Rouen ; Hainaut : Jean le Bel ; sinon « Un clerc de votre conseil »). Seul le sous-titre change, la voix synthétisée reste la même.
6. Contrastes : `INK_FADED` assombri (3,6:1 -> ~5,6:1 sur parchemin) ; sous-titre de « Continuer » plus clair et droit ; menu principal sans le cartouche « Cent Ans » cuit dans l'image (fond `menu_map_plain.jpg`).
7. Typographie : 114 apostrophes droites converties (`tools/typo_apostrophes.py`) ; « Abu l-Hasan » partout ; bundle du codex régénéré.
8. Domaine : `DomainPercent` n'affiche plus « 1000 % » au tour 0 (départ = domaine courant tant que non enregistré). Objectifs d'acte « acquis à conserver » non cochés (`held`) donc plus de « 4 réussis » au tour 0.

## Infirmé / reporté
- Prestige 90 (data) contre 0 (HUD) : confirmé, le champ `prestige` des fichiers factions n'est lu nulle part. Le brancher sur le souverain (essayé) change le comportement de l'IA (test `recruits_join_a_small_army...` casse) : décision d'équilibrage, reportée.
- `text_scale 0.9` : défaut réel 1.0 (code et settings), pas de changement.
- Carte du choix de faction sans noms ni légende : la survol + la liste de noms existent déjà ; étiquettes sur carte reportées (fonction).
- « Edimbourg » du fond de menu et cartouche : texte cuit dans `menu_map.jpg` (script `tools/cent_ans_tools/menu_art.py`), non régénéré.
- « Gediminas » -> « Gédimin » : forme lituanienne cohérente dans les listes de noms, non touchée.
- Entrées de personnages futurs (Du Guesclin né en 1320) restent listées en 1337 : l'ère commence avant.
- Réserve de recrutement : non touchée (lot ECO).
