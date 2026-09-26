# DA — Direction artistique (orchestration)

Demande du joueur (25/09 soir) : « tu es le directeur artistique, que manque-t-il au jeu
comparé à Total War et Crusader Kings ? » puis « ok pour la recommandation ».
Bible : `docs/design/2026-09-25-bible-da.md` (à lire avant tout lot DA).
Branche d'orchestration : `feat/da-direction-artistique` (worktree `../gp-da`).
Budget propre : **50 $** sur la clé OpenRouter personnelle du joueur (relevé de 15 $ le 25/09 ~23 h) (section « Direction artistique » de `docs/budget.md`).

## Lots

| Lot | Objet | Dépend de | Coût prévu | État |
|---|---|---|---|---|
| DA0 | Bible DA + captures d'état `docs/img/da/` | — | 0 $ | fait |
| DA1 | Armoiries des maisons + héraldique sur les figurines (surcot, écu, caparaçon ; les nobles portent les armes de leur seigneur) | DA0 | 0 $ (procédural) | vague 1 |
| DA2 | Portraits vivants : banque d'archétypes pour les personnages nés en jeu, vieillissement, marques (couronne, blessure, maladie, deuil), cadres par rang | DA0 | ≤ 8 $ | vague 1 |
| DA3 | Langage unique des marqueurs de ville sur la carte (forme = type, écu = propriétaire, taille = rang) | DA0, après ZG4b | 0 $ | vague 2 |
| DA4 | Musique : thèmes par culture, bataille en couches pilotée par l'intensité | DA0 | ≤ 3 $ | vague 2 |
| DA5 | Famille d'icônes d'action unique (trait d'encre) | DA0 | ≤ 3 $ | vague 2 |
| DA6 | Bataille rapprochée : herbe/cultures à lisière douce, arbres lointains | après EP6 | 0 $ | vague 3 |

Retirés de la recommandation initiale après inventaire : accessoires d'époque (pieux, pavois,
vignes : déjà faits en BV1 ou en cours dans EP6) ; « beauté de la carte » (déjà au niveau, réduit
à DA3 lisibilité).

## Journal

- 25/09 ~22 h : inventaire, captures à jour depuis `../gp-da` (l'arbre principal avait un cache
  de classes Godot périmé : `FineGeoLayer` introuvable ; import refait dans le worktree), bible
  écrite. Prochaine étape : lancer DA1 et DA2 (agents en worktree).
- 25/09 ~22 h 30 : vague 1 lancée — DA1 (héraldique, wip `da1-heraldique.md`) et DA2 (portraits
  vivants, ≤ 8 $, wip `da2-portraits-vivants.md`), agents en worktrees, chacun fusionne d'abord
  `feat/da-direction-artistique` pour la bible. L'orchestrateur fusionne (jamais les agents).
  Vague 2 (DA3 marqueurs après ZG4b, DA4 musique, DA5 icônes) après retour de la vague 1.
- 25/09 ~23 h : **PAUSE demandée par le joueur** ; reprise dans un autre terminal. DA1 et DA2
  ont reçu l'ordre de commiter un `wip:` et de s'arrêter. Reprise : `git worktree list | grep
  agent` pour retrouver leurs worktrees/branches, lire `docs/wip/da1-heraldique.md` et
  `docs/wip/da2-portraits-vivants.md` sur ces branches, vérifier la dépense DA2 dans
  `docs/budget.md` avant toute génération.
- 25/09 ~23 h : pause confirmée par les deux agents (rien fusionné dans main).
  - DA1 : branche `worktree-agent-addd87848de848c53` (worktree
    `.claude/worktrees/agent-addd87848de848c53`), commit `6fc9d9fc`. Fait : `data/heraldry/houses.json`
    (51 maisons, sources, `arms_of`, `vassal_of`, `badges`) + schéma. 3 substitutions (Artevelde,
    Béhuchet, Le Bel), Petrarca incertain. Reste : test du schéma, grammaire `heraldry.py`
    (écus de faction inchangés à l'octet), `house_heraldry_texture` + interface, atlas
    `sampler2DArray` dans le shader skinné, captures, A/B perf, ADR.
  - DA2 : branche `worktree-agent-ade9b18af852c5bfe` (worktree
    `.claude/worktrees/agent-ade9b18af852c5bfe`), commit `495470d9`, **0 $ dépensé**. Fait :
    `data/portraits/archetypes.json` + schéma (122 archétypes + 25 variantes âgées), générateur
    `portrait_archetypes.py` (`--dry-run` : 147 images ≈ 6,69 $), captures avant `docs/img/da2/`.
    Reste : pytest, sonde 3 images, génération, tout le GDScript, captures après, ADR.
- 25/09 ~23 h 15 : reprise dans un nouveau terminal (clé OpenRouter personnelle du joueur, 50 $).
  Le joueur veut valider la direction artistique **avant** de relancer DA1/DA2.
- 25/09 ~23 h 40 : DA validée par le joueur (bible § 0) ; planche `docs/img/da/planche/planche_da.jpg`
  validée (6 portraits sondes, bouton cloche, icônes encre ; 0,36 $). Liste d'assets validée
  (≈ 11,5 $ prévus : DA1 0 $, DA2 ≈ 6,4 $, DA5 boutons ≈ 0,7 $ + icônes ≈ 3,6 $, DA3 ≈ 0,7 $,
  DA4 musique libre 0 $, DA6 0 $). Relance DA1 + DA2, lancement DA5 + DA4.
- 25/09 ~23 h 45 : **autonomie totale** accordée par le joueur pour tous les lots DA. En cours :
  DA1 et DA2 repris (mêmes worktrees), DA5 boutons + icônes (≤ 5 $), DA3 marqueurs (≤ 1,5 $,
  ZG4b fusionné), DA4 musique libre (Sonnet, 0 $). DA6 attend la fin d'EP6. Fusions : par
  l'orchestrateur dans `feat/da-direction-artistique` (../gp-da), puis ff dans main.
- 26/09 : DA2 code terminé (branche agent, `fc968b5f`, ADR 0063, pytest 569 + smoke OK). La
  génération des 141 portraits a été **refusée par le garde-fou de permissions de l'agent** :
  à lancer par le joueur (`uv run --project tools cent-ans assets portrait-archetypes --envelope 7.6`
  dans le worktree de DA2, ≈ 6,42 $), puis import Godot, commit des images, captures `apres`,
  fusion. DA3/DA5 risquent le même blocage pour leurs images.
- 26/09 : **DA4 dans main** (`fa3cee8d`) avec revue DA : rendus MIDI (Machaut, Solage, Landini,
  Binchois) passés en repli (bible § 9, test `test_midi_renders_are_fallback_only`) ; test des
  playlists adapté. Dette : vrais enregistrements libres d'Ars nova pour France/Italie.
- 26/09 : **DA1 fusionné** (ADR 0064) ; vérifié en bataille réelle après EP6
  (`docs/img/da1/apres_bataille_reelle.jpg`), smoke + pytest 601 OK. Suites : **DA1b** meubles
  animaux (lions, léopards, aigles) trop schématiques dans `heraldry.py` ; armes du général sur
  les étendards EP5.
- 26/09 : main = `f98386ce` (DA0 + DA1 + DA4). Worktrees DA1/DA4 supprimés. Lancés : **DA6**
  (végétation de bataille, EP6 fusionné) et **DA1b** (meubles héraldiques SVG libres + armes du
  général sur les étendards EP5). En cours : DA3, DA5. DA2 : génération des 141 portraits en
  attente du joueur.
- 26/09 : **DA5 fusionné** (ADR 0065) : 78 icônes à l'encre (110 identifiants), 15 médaillons
  (cloche validée), 4,46 $ réels. Smoke + pytest 611 OK. Suites : icônes d'entité (unités,
  bâtiments, techniques…) encore en game-icons → miniatures peintes (bible § 8) ; marqueurs
  d'unité en bataille mélangés ; médaillons journal/research non branchés.
- 26/09 : **DA3 fusionné** (ADR renuméroté **0066**, le 0060 étant la musique) : 12 pictogrammes
  peints + atlas, écu du contrôleur, tri par rang selon le zoom dans les données, 0,59 $.
  Cumul DA : 5,43 $. Suites : chevauchements en régions denses, légende petite, écus de maison
  dans l'atlas.
- 26/09 : **DA1b fusionné** (ADR 0064 révisé) : 12 meubles SVG libres (PD/CC0/CC BY 4.0) rendus
  par rôle de couleur (`resvg-py`), 80 écus et 240 étoffes régénérés, bannière de maison du
  général et de sa retenue sur les étendards EP5 (oriflamme en second porte-étendard). Smoke +
  pytest 666 OK. Limites : cadrage EP5 `--standard-shot=foot` côté France, petits châteaux
  polygonaux, dauphin pâmé.
- 26/09 : **DA6 fusionné** (ADR renuméroté **0067**) : lisières ondulées et fondues, touffes à 4
  cartes cintrées, feuillus procéduraux par essence (LOD + imposteurs), texture de détail du sol,
  décor désaturé ; +1,5 à +3,5 % de coût, 0 $. Smoke + pytest 668 OK. Capture combinée DA1/DA1b/
  DA5/DA6 : `docs/img/da/etat_2609_bataille_combinee.jpg`. Limites : saturation automne/bocage
  > 35 % (lumière `BattleAtmosphere`), blé qui ne suit pas la saison.
- 26/09 : **DA5b fusionné** (ADR 0065 § DA5b) : 156 miniatures d'entité (101 dérivées des
  illustrations, 55 générées, 2,53 $), marqueurs et cartes d'unité unifiés. Budget DA recalculé
  après conflit : **7,96 $**. Smoke + pytest 681 OK. Reste du chantier : **DA2** (141 portraits,
  ≈ 6,4 $, génération à lancer par le joueur) ; suites listées plus haut (Ars nova réelle,
  saturation automne, traits sans miniature, chevauchements de marqueurs).
- 26/09 matin : **DA2 — génération faite** par l'orchestrateur à la demande du joueur (« ne valide
  plus par moi ») : 141 portraits, 0 échec, **6,33 $** (retries par image ajoutés à
  `portraits.generate`). Banque complète : 122 archétypes + 25 variantes âgées. Budget DA
  recalculé après conflit : **14,29 $** (14 lignes, aucune dédoublonnée).
  Fusion de DA2 dans `feat/da-direction-artistique` (conflits DA1/DA2 résolus : portrait vivant
  → armes de la maison → faction → initiales ; cadre DA2 avec armes de maison ; fiche de
  personnage : déclaration `house` rétablie, écu toujours affiché). Test DA2 adapté à la banque
  complète. Correctifs après captures : miroir des visages types selon l'id (anti-clones),
  vignettes de cour 64 px. Dernier commit `b8cc931f`, **pas encore dans main**.
- **PAUSE (26/09 matin)**, reprise dans une autre session. Prochaines étapes :
  1. Défaut ouvert : dans l'arbre familial (1367, `docs/img/da2/apres-arbre.png`), **Mahaut de
     Valois (22 ans) et Gille de Valois (17 ans) n'ont que l'écu**. Le code de l'arbre est bon
     (`LivingPortrait.texture_for`) ; soupçon : champ manquant dans le nœud d'arbre (`sex`,
     `age`/`birth_year`, `faction`) ou rang résolu vers une case sans image. Diagnostic :
     script qui déroule 120 tours puis affiche `LivingPortrait.resolve(n, context_for(n))` pour
     ces nœuds (lent sur la machine partagée : > 10 min).
  2. L'arbre n'applique pas le miroir anti-clones (il dessine `_texture` directement).
  3. Relancer smoke + `da2_living_portrait_test.gd` + pytest, recapturer (`da2_screenshot.gd`),
     puis fusionner main dans la branche, reconstruire la dylib (`CARGO_TARGET_DIR` partagé),
     et **ff main**.
  4. Nettoyer les worktrees d'agents fusionnés (DA2 : `.claude/worktrees/agent-ade9b18af852c5bfe`).
- 26/09 après-midi : **reprise et fin de DA2.** Fusion de main (357 commits ; conflit
  `portraits.generate` : nouvelles tentatives par image + coût réel des refus facturés de main,
  test adapté). Défaut « Mahaut/Gille sans visage » : import Godot incomplet, disparu après
  réimport. Nouveau défaut corrigé : **clones de fratrie** → miroir partagé (`LivingPortrait.mirrored`,
  bit de `fnv1a`) dans le cadre et l'arbre, et **80 visages de plus** pour les cases peuplées
  (nobles/chevaliers hommes 4, nobles femmes 3, enfants 3 ; **3,59 $**). Banque : 202 archétypes
  + 25 variantes âgées. Budget DA : **17,88 $**. Smoke + test DA2 OK. Note : `da2_screenshot.gd
  --out=` est relatif au dossier `game/` (utiliser le défaut).
- 26/09 soir : **vague DA7** lancée (4 agents en worktree, fusion via `feat/da-direction-artistique`
  puis ff main) : **DA7a** vrais enregistrements libres d'Ars nova (0 $, `docs/wip/da7a-ars-nova.md`),
  **DA7b** saturation automne/bocage ≤ 35 % (0 $, `docs/wip/da7b-saturation.md`), **DA7c** icônes
  de traits (≤ 4 $, `docs/wip/da7c-icones-traits.md`), **DA7d** chevauchements de marqueurs de
  ville (0 $, `docs/wip/da7d-chevauchements.md`). Chaque lot complète l'ADR DA existant
  (0060, 0067, 0065, 0066).
- 26/09 soir : **DA7b fusionné** (main `1f723a84`, ADR 0067 § DA7b) : automne 44,7 → 32,7 %, bocage
  haies 39,1 → 33,7 %, toutes les vues ≤ 35 % ; `battle_seasons` + `shadow_saturation` dans
  `data/fx/atmosphere.json`, outil `scene_saturation.py`. DA7d fini (664 → 0 chevauchements)
  mais en conflit avec le dé-encombrement de noms DC4 dans `settlement_layer.gd` : renvoyé à
  l'agent pour unifier les deux avant fusion.
