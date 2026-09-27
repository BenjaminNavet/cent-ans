Première préversion jouable sous **Windows 10/11 64 bits** (ADR 0087).

## Installer et jouer

1. Télécharger **`Cent.Ans.Windows.zip`** (≈ 770 Mo) ci-dessous.
2. Clic droit → **Extraire tout…** (ne pas lancer le jeu depuis l'intérieur du zip).
3. Ouvrir le dossier `Cent Ans` et double-cliquer sur **`Cent Ans.exe`**.
4. Windows affiche sans doute « Windows a protégé votre ordinateur » : l'exécutable n'est pas signé.
   Cliquer sur **Informations complémentaires → Exécuter quand même**.

Rien d'autre à installer. Il faut une carte graphique compatible **Vulkan** ou **Direct3D 12**
(la plupart des PC depuis 2016).

En cas de problème, lancer **`Cent Ans.console.exe`** : il garde une fenêtre avec le journal du
jeu. Les sauvegardes et réglages sont dans `%APPDATA%\Cent Ans`.

## Limites de cette préversion

- **Pas encore essayée sur un vrai PC** : la simulation passe le test automatique sur Windows
  (GitHub Actions), mais le rendu et les performances sur PC n'ont pas été mesurés.
- **Relief fin non inclus** (≈ 3 Go, au-delà de la limite de 2 Go par fichier de GitHub) :
  le zoom rapproché est limité, et un avis s'affiche en jeu.
- Pas d'icône personnalisée sur l'exécutable.
