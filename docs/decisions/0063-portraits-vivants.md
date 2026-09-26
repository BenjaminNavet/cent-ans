# ADR 0063 — Portraits vivants (lot DA2)

Date : 2026-09-25. Statut : accepté (banque d'images incomplète, voir « Limites »).

## Contexte
Les 88 portraits fixes (256 px) figent les figures historiques à leur âge de 1337. Les
personnages nés en jeu n'ont que l'écu de leur faction ou leurs initiales. Comme la partie peut
durer jusqu'en 1453, la cour finit par ne plus avoir de visages (bible DA § 10, écart n° 1).

## Décision
1. **Banque d'archétypes pilotée par les données** (`data/portraits/archetypes.json`, schéma
   `portrait_archetypes.schema.json`). Chaque case est définie par un rang d'affichage (enfant,
   souverain ou consort, grand noble, chevalier ou capitaine, prélat, bourgeois), un sexe, une
   tranche d'âge (enfant ≤ 15 ans, jeune ≤ 29, adulte ≤ 49, âgé) et une aire d'habit
   (France/Bourgogne/Pays-Bas, Angleterre/Écosse, Ibérie, Italie/Empire ; chaque faction
   appartient à une seule aire). Une case compte 1 ou 2 visages ; la banque totalise
   122 images. Les images sont générées par `cent-ans assets portrait-archetypes`
   (`portrait_archetypes.py`), qui réutilise `portraits.STYLE`, `portraits.generate` (enveloppe
   du lot, plafond global, une ligne par passe dans `docs/budget.md`) et `budget.py`. Format :
   JPEG 512 × 512 (bible § 5), 70 à 95 Ko par image. Les archétypes ne portent pas
   d'armoiries : le jeu peint l'écu de la faction dans le cadre.
2. **Attribution déterministe.** Le visage est l'index `FNV-1a(id) mod visages` de la case.
   FNV-1a est codé en GDScript, ce qui le rend indépendant de `String.hash()` et de la version
   du moteur. Un même personnage garde donc la même image d'une sauvegarde à l'autre. D'une
   tranche d'âge à l'autre, il garde aussi la même description de visage (cheveux, forme du
   visage, yeux), ce qui fait une « lignée » de visage : l'index i renvoie toujours à la
   description i de `faces`. Quand une case manque, des replis s'appliquent dans cet ordre :
   rang, tranche, aire `any`, puis les autres aires.
3. **Vieillissement.** Le portrait fixe d'une figure historique vaut pour une tranche de base :
   son âge en 1337, avec un minimum de 20 ans pour les figures nées plus tard. Les 25 grandes
   figures listées dans `aged_variants` ont des variantes `aged/<id>_<tranche>.jpg`, générées
   avec le portrait de 1337 envoyé comme référence de ressemblance
   (`openrouter.request_image(images=…)`). Elles affichent la plus récente image dont la
   tranche ne dépasse pas leur âge. Les autres figures passent à un archétype dès qu'elles
   changent de tranche, sauf si la banque n'a pas d'archétype de leur aire : elles gardent
   alors leur portrait fixe plutôt que d'emprunter l'habit d'une autre aire.
4. **Rang d'affichage**, déterminé côté rendu à partir de l'état exposé par la façade, sans
   nouvel accesseur Rust :
   - le dirigeant et son conjoint (via `get_family_tree(id, 0, 0)` → `ruler`) ;
   - sinon le rôle de données de la figure historique (`GameDataStore.get_character.role`) ;
   - sinon des mots-clés du titre, l'armée commandée et la maison régnante ;
   - sinon un rang par défaut selon le sexe.
   Les règles sont dans `rank_rules`. C'est un choix de présentation, pas une règle de jeu.
5. **Marques procédurales** (`PortraitFrame`, shader `portrait_marks.gdshader`), sans nouvelle
   génération :
   - cadre selon le rang : or et azur (souverain), or (grand noble), encre et or (chevalier),
     or et pourpre (prélat), encre (bourgeois), avec les teintes canoniques de la bible § 3.1 ;
   - couronne ou mitre si l'image n'en porte pas (un héritier devenu roi, par exemple) ;
   - grisaille pour un défunt ;
   - pâleur pour un personnage maladif (`trait_sickly`) ;
   - trait de gueules pour un blessé ;
   - bandeau de sable pour le deuil (conjoint ou parent mort depuis moins de `mourning_years`) ;
   - barreaux pour un captif.
   L'arbre familial garde son propre médaillon, grisé et couronné.
6. **Branchement.** `PortraitLoader.portrait_texture(id)` renvoie le portrait vivant pendant
   une campagne, ce qui couvre l'arbre, le sceau du général, le HUD et le dialogue de bataille,
   ainsi que le choix de faction (portrait fixe hors campagne).
   `PortraitLoader.overlay_portrait` pose un `PortraitFrame` pour la cour, la fiche de
   personnage et la rançon. Replis dans l'ordre : écu de la faction, puis initiales.

## Conséquences
- Tout personnage a un visage de son rang, de son âge et de son aire, dès que la banque est
  complète.
- Un nouvel axe ou de nouveaux visages s'ajoutent dans les données ; il suffit ensuite de
  relancer la commande, qui ne génère que les images manquantes.
- La résolution appelle `get_character` à chaque portrait, et le contexte de rang est mis en
  cache par faction et par date. Le coût est négligeable pour la cour et l'arbre (au plus
  120 nœuds).

## Limites
- Le 25/09, seules les 6 images de sonde (validées par le joueur) existent. La passe payante
  des 141 images restantes (environ 6,4 $) a été refusée par le garde-fou des transactions de
  l'agent : il faut la relancer avec l'autorisation du joueur. En attendant, les replis
  choisissent la sonde la plus proche : chevalier anglais adulte pour les hommes, rien pour
  les femmes.
- Les nœuds de l'arbre ne portent ni traits ni captivité : dans l'arbre, seule la grisaille des
  défunts s'applique.
