# Windows and Linux Release Signing Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `executing-plans` to implement this plan
> task-by-task. Use `aside-browser` for every account, portal, billing, identity-validation and
> GitHub-settings operation.

**Goal:** Sign and verify every public SkillReg Windows and Linux release artifact while keeping
Axel out of the operational flow except for the final payment authorization.

**Architecture:** Keep the existing Tauri updater signature as the application-update trust layer.
Add Microsoft Azure Artifact Signing Public Trust for Windows Authenticode and an OpenPGP signing
subkey for Linux artifacts. GitHub Actions builds on native runners, fails closed when a signature
is absent, verifies every artifact before publication, creates provenance attestations and publishes
only after all platform gates pass.

**Tech Stack:** Tauri v2, Rust, pnpm, GitHub Actions, Microsoft Entra ID, Azure Artifact Signing,
GitHub OIDC, Authenticode/SignTool, OpenPGP/GnuPG, AppImage, RPM, SHA-256, Tauri updater signatures.

---

## 1. Résultat attendu

À la fin de ce plan :

- Windows affiche l’éditeur juridique vérifié de Kairia, jamais « Unknown publisher » ;
- l’exécutable SkillReg, l’installateur NSIS et le MSI portent une signature Authenticode SHA-256
  horodatée ;
- l’AppImage Linux contient une signature OpenPGP vérifiable ;
- le RPM porte une signature OpenPGP valide ;
- le `.deb`, l’AppImage, le RPM et les autres artefacts publics figurent dans un manifeste
  `SHA256SUMS` lui-même signé ;
- les artefacts d’updater Windows et Linux conservent leur signature Tauri existante ;
- les artefacts publics disposent d’une attestation de provenance GitHub ;
- `latest.json` ne référence que des archives d’updater vérifiées ;
- la release reste en brouillon tant qu’une gate de signature ou de QA échoue ;
- aucune clé privée, valeur de paiement, pièce d’identité ou donnée secrète n’entre dans le dépôt,
  les logs ou les captures ;
- toute opération web est effectuée par Aside ;
- Axel n’intervient que lorsque le portail présente la validation finale du paiement.

## 2. Décisions figées

### 2.1 Windows

La solution cible est **Azure Artifact Signing Basic, Public Trust** :

- région Azure : Europe, avec préférence pour `West Europe` si le service est disponible au moment
  de la création ;
- SKU : `Basic`, jamais `Premium` ;
- profil : organisation Kairia, sous la raison sociale exacte validée par Microsoft ;
- coût cible : `9,99 USD/mois` hors éventuelles taxes et conversion ;
- authentification CI : GitHub OIDC vers Microsoft Entra, sans secret client longue durée ;
- rôle Azure : `Artifact Signing Certificate Profile Signer`, limité au profil de certificat ;
- hash : SHA-256 ;
- horodatage RFC 3161 : `http://timestamp.acs.microsoft.com`.

Un certificat OV/EV traditionnel n’est pas créé. Le Microsoft Store n’est pas requis dans ce lot.

### 2.2 Linux

Le premier lot conserve la distribution GitHub Releases existante :

- AppImage signé avec une sous-clé OpenPGP ;
- RPM signé avec la même identité Kairia lorsque la compatibilité le permet ;
- `.deb` protégé par le manifeste de checksums signé ;
- `SHA256SUMS` et `SHA256SUMS.asc` publiés avec la release ;
- clé publique et empreinte publiées sur `skillreg.dev` ;
- signature Tauri de l’updater maintenue séparément.

Un dépôt APT/RPM signé est un lot ultérieur. Il n’est pas nécessaire pour rendre la release actuelle
vérifiable.

### 2.3 Clé de l’updater Tauri

La clé Tauri existante n’est pas remplacée :

- les clients déjà installés épinglent sa clé publique ;
- une rotation non planifiée casserait les mises à jour ;
- `TAURI_SIGNING_PRIVATE_KEY` et son mot de passe restent des secrets GitHub ;
- leur présence est vérifiée sans jamais afficher leur valeur.

### 2.4 Publication

La release est construite et vérifiée automatiquement. Elle n’est rendue publique que si :

1. les quatre builds passent ;
2. les signatures Tauri existent ;
3. Authenticode est valide sur tous les artefacts Windows ;
4. les signatures Linux et checksums sont valides ;
5. les attestations ont été créées ;
6. `latest.json` contient toutes les plateformes attendues ;
7. les tests d’installation et d’update ont été consignés.

## 3. Contrat d’autonomie Aside

### 3.1 Répartition des responsabilités

| Action | Exécutant | Intervention Axel |
| --- | --- | --- |
| Lire les pages officielles | Aside | Aucune |
| Ouvrir ou retrouver les comptes Microsoft/Azure | Aside | Aucune |
| Créer le tenant, l’abonnement ou les ressources Azure | Aside | Aucune |
| Remplir les informations Kairia | Aside | Aucune |
| Ouvrir les emails de vérification | Aside | Aucune |
| Charger les justificatifs disponibles | Aside | Aucune |
| Créer l’application Entra et la fédération OIDC | Aside | Aucune |
| Configurer variables, secrets et environnement GitHub | Aside | Aucune |
| Préparer l’écran de paiement | Aside | Aucune |
| Saisir ou autoriser le moyen de paiement | Axel | **Seule intervention prévue** |
| Vérifier la confirmation après paiement | Aside | Aucune |
| Modifier et tester le code | Codex | Aucune |
| Lancer les workflows et analyser les résultats | Codex | Aucune |
| Publier ou annuler la release | Codex | Aucune |

### 3.2 Usage des deux surfaces Aside

Utiliser `aside exec` pour les parcours complets :

- Microsoft Entra ;
- Azure Portal ;
- Artifact Signing ;
- boîte email de vérification ;
- paramètres GitHub ;
- récupération du statut des validations.

Utiliser `aside repl` pour :

- attacher l’onglet réellement ouvert ;
- lire un formulaire avec `snapshot()` ;
- vérifier les montants et le SKU avant paiement ;
- contrôler l’état après chaque mutation ;
- capturer uniquement des preuves expurgées ;
- confirmer que GitHub et Azure ont accepté la configuration.

Avant toute session :

```bash
aside --help
aside exec --help
aside repl --help
```

### 3.3 Règle de paiement

Aside doit :

1. remplir les formulaires non financiers ;
2. sélectionner uniquement `Artifact Signing Basic` ;
3. vérifier que le prix affiché correspond à environ `9,99 USD/mois` ;
4. s’arrêter avant toute action qui autorise une carte, un prélèvement ou un achat ;
5. annoncer à Axel le montant exact, la périodicité et le bénéficiaire ;
6. laisser Axel saisir ou confirmer le paiement dans l’onglet Aside ;
7. reprendre la session après confirmation ;
8. vérifier que la ressource est active et que le SKU est `Basic`.

Si le prix, la périodicité ou le SKU diffère, ne rien acheter et demander une décision.

### 3.4 Exceptions imposées par le fournisseur

L’objectif est une unique intervention au paiement. Aside doit utiliser les sessions connectées,
l’autofill, les emails accessibles et les documents Kairia déjà disponibles.

Il est interdit de contourner :

- MFA physique ;
- biométrie ;
- captcha demandant explicitement une personne ;
- déclaration légale personnelle ;
- signature électronique nominative.

Si Microsoft impose l’un de ces contrôles et qu’Aside ne peut pas le satisfaire légitimement, la
session s’arrête et le point est signalé comme une exception fournisseur. Il ne doit jamais être
simulé ou contourné.

### 3.5 Données et preuves

- Ne jamais afficher une clé privée, un token, une carte ou un document d’identité dans les logs.
- Ne jamais enregistrer de capture d’un champ secret.
- Ne jamais télécharger un justificatif dans le dépôt.
- Conserver uniquement : nom des ressources, région, SKU, statut, empreinte publique et horodatage.
- Les captures temporaires vivent dans `/tmp` et sont supprimées après la rédaction du compte
  rendu expurgé.

## 4. Budget et calendrier

### Budget

| Poste | Coût |
| --- | ---: |
| Azure Artifact Signing Basic | 9,99 USD/mois |
| OpenPGP Linux | 0 |
| Signature Tauri | 0 |
| GitHub Actions, si dépôt public et runners standards | 0 |
| GitHub attestations, si dépôt public | 0 |
| Hébergement GitHub Releases | 0 coût incrémental attendu |
| Enveloppe prudente annuelle | 150 EUR taxes et change inclus |

### Temps actif

| Travail | Estimation |
| --- | ---: |
| Préparation scripts/tests locaux | 2 h |
| Parcours Azure/GitHub avec Aside | 1 h |
| Intégration CI Windows | 2 h |
| Intégration CI Linux | 1 h 30 |
| QA et documentation | 1 h 30 |
| Total actif cible | 8 h |

La validation d’identité Microsoft peut ajouter plusieurs jours calendaires sans travail actif.

## 5. État initial constaté

Les éléments suivants existent déjà :

- `.github/workflows/release.yml` construit macOS arm64/x64, Linux x64 et Windows x64 ;
- le workflow collecte `.dmg`, `.deb`, `.rpm`, `.AppImage`, `.exe`, `.msi` et archives d’updater ;
- `TAURI_SIGNING_PRIVATE_KEY` génère les signatures Tauri ;
- `latest.json` contient déjà les entrées Linux et Windows ;
- `src-tauri/tauri.conf.json` utilise SHA-256 ;
- `scripts/check-release-notarization.sh` protège les hooks Apple.

Les lacunes à fermer :

- `certificateThumbprint` est vide et aucun `signCommand` Windows n’est configuré ;
- `timestampUrl` est vide ;
- les artefacts Windows ne sont pas vérifiés avant collecte ;
- aucune clé GPG Linux n’est importée dans le runner ;
- `SIGN`, `APPIMAGETOOL_FORCE_SIGN` et les variables RPM ne sont pas définies ;
- aucun checksum signé n’est publié ;
- le job de publication ne vérifie pas les signatures système ;
- la release créée en brouillon n’a pas de gate explicite de passage en public ;
- le workflow utilise `pnpm install` sans `--frozen-lockfile`.

## 6. Variables, secrets et ressources

### Ressources Azure

Noms recommandés, à ajuster uniquement si Azure impose l’unicité :

```text
Resource group:             skillreg-signing-prod
Artifact Signing account:  skillreg-signing-prod-<suffixe>
Identity validation:       Kairia
Certificate profile:       skillreg-public-trust
Entra application:         skillreg-github-release-signing
GitHub environment:        release-signing
```

### Variables GitHub non secrètes

```text
AZURE_TENANT_ID
AZURE_SUBSCRIPTION_ID
AZURE_CLIENT_ID
AZURE_ARTIFACT_SIGNING_ENDPOINT
AZURE_ARTIFACT_SIGNING_ACCOUNT
AZURE_ARTIFACT_SIGNING_PROFILE
WINDOWS_EXPECTED_PUBLISHER
LINUX_GPG_KEY_ID
LINUX_GPG_FINGERPRINT
```

Ces identifiants ne sont pas des mots de passe. Ils restent néanmoins dans les variables de
l’environnement GitHub plutôt que dans le code afin de simplifier un changement de compte.

### Secrets GitHub

```text
TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY_PASSWORD
LINUX_GPG_PRIVATE_KEY_B64
LINUX_GPG_PASSPHRASE
```

Aucun secret Windows n’est créé dans le chemin nominal OIDC.

### Repli Windows

Si le signer appelé par Tauri ne sait pas consommer l’identité OIDC lors du build :

1. vérifier d’abord l’authentification via `DefaultAzureCredential` et Azure CLI ;
2. tester l’outil Microsoft `sign` comme `signCommand` ;
3. seulement si ces deux chemins échouent, créer avec Aside un secret client Entra limité à
   90 jours ;
4. l’écrire directement dans le secret GitHub `AZURE_CLIENT_SECRET` sans l’afficher ;
5. documenter la date de rotation ;
6. ouvrir une tâche de suppression dès que l’OIDC fonctionne.

Le repli ne doit pas être activé silencieusement.

## 7. Critères d’acceptation

### Windows

- `Get-AuthenticodeSignature` retourne `Valid` pour l’exécutable, le NSIS et le MSI.
- `signtool verify /pa /all /v` réussit pour chaque fichier.
- le sujet du certificat correspond à `WINDOWS_EXPECTED_PUBLISHER`.
- une contresignature RFC 3161 est présente.
- l’installation affiche « Éditeur vérifié » avec le nom Kairia attendu.
- le binaire installé porte lui aussi une signature valide.
- l’update depuis la dernière version stable installe l’artefact signé.

### Linux

- la signature AppImage est présente et validée par l’outil AppImage officiel ;
- `rpm --checksig` retourne une signature valide ;
- `gpg --verify SHA256SUMS.asc SHA256SUMS` réussit ;
- chaque artefact publié est listé dans `SHA256SUMS` ;
- l’empreinte correspond à celle publiée sur `skillreg.dev` ;
- l’update Tauri vérifie la signature existante et installe le nouvel AppImage.

### Supply chain

- aucun artefact unsigned ne peut atteindre le job `publish` ;
- une attestation GitHub existe pour chaque artefact public ;
- les actions critiques sont épinglées sur une version majeure validée ou, idéalement, un SHA ;
- les permissions GitHub sont minimales par job ;
- `id-token: write` n’est accordé qu’au job Windows de signature et aux attestations ;
- aucun secret n’apparaît dans les logs ;
- le tag, la version Tauri et la version `package.json` correspondent.

### Interaction

- le journal d’exécution ne mentionne qu’une seule demande planifiée à Axel : le paiement ;
- chaque mutation de compte est confirmée par un snapshot Aside après action ;
- aucune donnée sensible n’est incluse dans les preuves.

## 8. Plan d’exécution détaillé

### Task 1: Geler le contrat de release

**Files:**

- Create: `scripts/check-release-signing.sh`
- Modify: `.github/workflows/ci.yml`
- Modify: `package.json`

**Step 1: écrire le test statique en échec**

Créer un script qui exige au minimum :

```bash
#!/usr/bin/env bash
set -euo pipefail

workflow_path="${1:-.github/workflows/release.yml}"

assert_contains() {
  local pattern="$1"
  rg -q --fixed-strings "$pattern" "$workflow_path" || {
    echo "Missing release signing hook: $pattern" >&2
    exit 1
  }
}

assert_contains "id-token: write"
assert_contains "release-signing"
assert_contains "AZURE_ARTIFACT_SIGNING_ENDPOINT"
assert_contains "Verify Windows Authenticode signatures"
assert_contains "APPIMAGETOOL_FORCE_SIGN"
assert_contains "Verify Linux signatures"
assert_contains "SHA256SUMS.asc"
assert_contains "attest"

echo "Release workflow contains Windows and Linux signing gates."
```

**Step 2: exécuter le test**

Run:

```bash
bash scripts/check-release-signing.sh
```

Expected: `FAIL` sur le premier hook absent.

**Step 3: ajouter le script npm**

Ajouter :

```json
"release:check-signing": "bash scripts/check-release-signing.sh"
```

**Step 4: appeler le test depuis la CI**

Ajouter dans `.github/workflows/ci.yml` :

```yaml
- name: Verify release signing contract
  run: pnpm release:check-signing
```

**Step 5: checkpoint**

Inspecter `git diff -- scripts/check-release-signing.sh package.json .github/workflows/ci.yml`.
Ne pas committer sans instruction explicite d’Axel.

### Task 2: Préparer le signer Windows

**Files:**

- Create: `src-tauri/tauri.windows.conf.json`
- Create: `src-tauri/scripts/sign-windows.ps1`
- Create: `src-tauri/scripts/test-sign-windows.ps1`

**Step 1: écrire le test du wrapper**

Le test doit vérifier :

- refus d’un chemin absent ;
- refus si une variable requise manque ;
- absence de valeur secrète dans l’erreur ;
- propagation d’un code de sortie non nul ;
- support des chemins contenant des espaces.

**Step 2: exécuter le test**

Run sur Windows :

```powershell
pwsh -File src-tauri/scripts/test-sign-windows.ps1
```

Expected: `FAIL` car `sign-windows.ps1` n’existe pas.

**Step 3: écrire le wrapper minimal**

Le wrapper reçoit un unique chemin Tauri, valide son existence puis appelle le signer :

```powershell
param(
  [Parameter(Mandatory = $true)]
  [string]$FilePath
)

$ErrorActionPreference = "Stop"

$required = @(
  "AZURE_ARTIFACT_SIGNING_ENDPOINT",
  "AZURE_ARTIFACT_SIGNING_ACCOUNT",
  "AZURE_ARTIFACT_SIGNING_PROFILE"
)

foreach ($name in $required) {
  if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name))) {
    throw "Missing required signing configuration: $name"
  }
}

$resolved = Resolve-Path -LiteralPath $FilePath

& sign code artifact-signing `
  --timestamp-url "http://timestamp.acs.microsoft.com" `
  --artifact-signing-endpoint $env:AZURE_ARTIFACT_SIGNING_ENDPOINT `
  --artifact-signing-account $env:AZURE_ARTIFACT_SIGNING_ACCOUNT `
  --artifact-signing-certificate-profile $env:AZURE_ARTIFACT_SIGNING_PROFILE `
  $resolved.Path

if ($LASTEXITCODE -ne 0) {
  throw "Artifact Signing failed for $($resolved.Path)"
}
```

Ne pas activer ce wrapper dans la configuration tant que le preflight OIDC n’a pas réussi.

**Step 4: créer la configuration Windows**

Configuration cible :

```json
{
  "bundle": {
    "windows": {
      "digestAlgorithm": "sha256",
      "signCommand": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sign-windows.ps1 %1"
    }
  }
}
```

Vérifier le répertoire de travail réel de `signCommand`. Ajuster le chemin du script si le test
Windows démontre qu’il est résolu depuis la racine plutôt que `src-tauri`.

**Step 5: exécuter le test**

Expected: tous les tests purement locaux passent ; l’appel Azure réel reste désactivé.

### Task 3: Ajouter la vérification Windows bloquante

**Files:**

- Create: `src-tauri/scripts/verify-windows-signatures.ps1`
- Create: `src-tauri/scripts/test-verify-windows-signatures.ps1`

**Step 1: écrire les tests en échec**

Couvrir :

- aucun artefact trouvé ;
- artefact unsigned ;
- signature invalide ;
- éditeur différent ;
- absence d’horodatage ;
- ensemble complet valide.

**Step 2: implémenter le vérificateur**

Le script doit trouver :

```text
src-tauri/target/release/skillreg-local.exe
src-tauri/target/release/bundle/nsis/*-setup.exe
src-tauri/target/release/bundle/msi/*.msi
```

Pour chaque artefact :

```powershell
$signature = Get-AuthenticodeSignature -FilePath $file
if ($signature.Status -ne "Valid") { throw "Invalid Authenticode signature: $file" }
if ($signature.SignerCertificate.Subject -notlike "*$env:WINDOWS_EXPECTED_PUBLISHER*") {
  throw "Unexpected publisher: $file"
}
& signtool verify /pa /all /v $file
if ($LASTEXITCODE -ne 0) { throw "signtool verification failed: $file" }
```

Analyser la sortie détaillée de `signtool` pour confirmer l’horodatage RFC 3161. Ne pas se
contenter du statut PowerShell.

**Step 3: tester**

Run :

```powershell
pwsh -File src-tauri/scripts/test-verify-windows-signatures.ps1
```

Expected: `PASS`.

### Task 4: Préparer l’identité OpenPGP Linux

**Files:**

- Create during execution outside Git: temporary GNUPGHOME under `/tmp`
- Publish later: `skillreg-website/public/skillreg-release-signing-key.asc`
- Modify later: GitHub environment secrets through Aside

**Step 1: créer la clé hors dépôt**

Créer une clé primaire de certification et une sous-clé dédiée à la signature. Utiliser une identité
publique de la forme :

```text
SkillReg Release Signing <security@skillreg.dev>
```

**Step 2: créer immédiatement le certificat de révocation**

Le certificat de révocation ne doit jamais être placé dans GitHub.

**Step 3: exporter**

Produire :

- clé publique ASCII ;
- sous-clé privée de signature, chiffrée ;
- certificat de révocation ;
- empreinte complète.

**Step 4: stocker**

- clé publique : site et documentation ;
- sous-clé CI : secret GitHub `LINUX_GPG_PRIVATE_KEY_B64` ;
- passphrase : secret GitHub `LINUX_GPG_PASSPHRASE` ;
- primaire et révocation : stockage sécurisé hors dépôt et hors runner.

Aside configure les secrets GitHub directement. La valeur ne doit pas être copiée dans le chat ou
les logs.

### Task 5: Ajouter les scripts de signature et vérification Linux

**Files:**

- Create: `scripts/generate-release-checksums.sh`
- Create: `scripts/verify-linux-signatures.sh`
- Create: `tests/release-signing.test.ts`

**Step 1: écrire les tests Node en échec**

Couvrir :

- répertoire vide ;
- artefact absent du manifeste ;
- checksum incorrect ;
- signature détachée absente ;
- empreinte inattendue ;
- RPM unsigned ;
- succès complet avec fixture.

**Step 2: générer les checksums**

Le script doit :

1. trier les fichiers de manière déterministe ;
2. exclure `SHA256SUMS` et sa signature ;
3. générer une ligne par artefact public ;
4. signer le manifeste en ASCII ;
5. échouer si aucun artefact n’existe.

Commande cible :

```bash
find artifacts -maxdepth 1 -type f \
  ! -name 'SHA256SUMS' \
  ! -name 'SHA256SUMS.asc' \
  -print0 \
  | sort -z \
  | xargs -0 sha256sum > artifacts/SHA256SUMS

gpg --batch --yes --armor --detach-sign \
  --local-user "$LINUX_GPG_KEY_ID" \
  --output artifacts/SHA256SUMS.asc \
  artifacts/SHA256SUMS
```

**Step 3: vérifier Linux**

Le script doit exécuter :

```bash
gpg --batch --verify artifacts/SHA256SUMS.asc artifacts/SHA256SUMS
sha256sum --check artifacts/SHA256SUMS
rpm --checksig artifacts/*.rpm
```

Il doit également valider la signature embarquée de l’AppImage avec l’outil officiel AppImage dont
la version et le checksum sont épinglés.

**Step 4: lancer les tests**

Run :

```bash
pnpm test:node
```

Expected: `PASS`.

### Task 6: Préparer le workflow sans activer la facturation

**Files:**

- Modify: `.github/workflows/release.yml`
- Modify: `.github/workflows/ci.yml`
- Modify: `src-tauri/tauri.windows.conf.json`

**Step 1: durcir les permissions**

Passer des permissions globales aux permissions par job :

```yaml
permissions:
  contents: read
```

Pour le job Windows de signature :

```yaml
permissions:
  contents: read
  id-token: write
```

Pour `publish` :

```yaml
permissions:
  contents: write
  attestations: write
  id-token: write
```

**Step 2: ajouter l’environnement Windows**

Le job Windows utilise :

```yaml
environment: release-signing
```

et des variables GitHub :

```yaml
env:
  AZURE_ARTIFACT_SIGNING_ENDPOINT: ${{ vars.AZURE_ARTIFACT_SIGNING_ENDPOINT }}
  AZURE_ARTIFACT_SIGNING_ACCOUNT: ${{ vars.AZURE_ARTIFACT_SIGNING_ACCOUNT }}
  AZURE_ARTIFACT_SIGNING_PROFILE: ${{ vars.AZURE_ARTIFACT_SIGNING_PROFILE }}
  WINDOWS_EXPECTED_PUBLISHER: ${{ vars.WINDOWS_EXPECTED_PUBLISHER }}
```

**Step 3: préparer l’authentification OIDC**

Ajouter `azure/login` avec :

```yaml
with:
  client-id: ${{ vars.AZURE_CLIENT_ID }}
  tenant-id: ${{ vars.AZURE_TENANT_ID }}
  subscription-id: ${{ vars.AZURE_SUBSCRIPTION_ID }}
```

Ne jamais ajouter `AZURE_CLIENT_SECRET` dans le chemin nominal.

**Step 4: préparer l’outil de signature**

Installer l’outil Microsoft retenu sur `windows-2022`, puis exécuter un diagnostic sans imprimer de
token :

```powershell
sign --version
az account show --query '{tenant:tenantId,subscription:id}' --output table
```

**Step 5: préparer Linux**

Sur le runner Linux :

- créer un `GNUPGHOME` temporaire ;
- importer la sous-clé depuis le secret ;
- configurer `SIGN=1` ;
- configurer `SIGN_KEY` ;
- configurer `APPIMAGETOOL_SIGN_PASSPHRASE` ;
- configurer `APPIMAGETOOL_FORCE_SIGN=1` ;
- fournir `TAURI_SIGNING_RPM_KEY` et sa passphrase ;
- supprimer le trousseau temporaire à la fin du job.

**Step 6: ne pas lancer de release**

Le workflow modifié reste non exécuté tant que les ressources Azure et les secrets Linux ne sont pas
prêts.

### Task 7: Vérifier Aside avant toute opération de compte

**Files:** aucun

**Step 1: inspecter la CLI**

```bash
aside --help
aside exec --help
aside repl --help
```

**Step 2: ouvrir une session Aside autonome**

Le prompt doit préciser :

```text
Préparer Azure Artifact Signing Basic pour SkillReg/Kairia.
Utiliser les comptes déjà connectés.
Ne jamais créer Premium.
Ne jamais confirmer un paiement.
S’arrêter sur le dernier écran avant paiement et rendre compte du montant exact.
Ne jamais afficher de secret, token, carte ou justificatif.
```

**Step 3: vérifier la session avec Aside REPL**

- lister les onglets ;
- attacher le bon onglet Azure ;
- prendre un snapshot interactif ;
- confirmer le compte et le tenant utilisés ;
- ne pas capturer les champs sensibles.

### Task 8: Préparer Azure jusqu’au paiement avec Aside

**Files:** aucun

**Step 1: retrouver ou créer le contexte Azure**

Aside :

- ouvre Azure Portal ;
- utilise le compte Kairia existant ;
- identifie l’abonnement et le tenant ;
- crée un abonnement seulement si nécessaire ;
- remplit les coordonnées légales disponibles.

**Step 2: préparer les ressources**

Aside prépare :

- resource group ;
- Artifact Signing account ;
- région européenne ;
- SKU Basic ;
- identité d’organisation Kairia.

**Step 3: contrôler l’écran final**

Avec `aside repl`, vérifier :

- `Basic` ;
- environ `9,99 USD/mois` ;
- aucune option Premium ;
- raison sociale attendue ;
- périodicité mensuelle.

**Step 4: arrêter**

Ne pas cliquer sur l’action qui engage le paiement.

### Task 9: Checkpoint unique de paiement Axel

**Files:** aucun

**Step 1: annoncer**

Présenter en une phrase :

```text
Azure Artifact Signing Basic — montant affiché : X — périodicité : Y — bénéficiaire : Microsoft.
L’écran est prêt pour ta confirmation.
```

**Step 2: laisser Axel agir**

Axel saisit ou confirme le paiement dans Aside. Aucun outil ne lit ou ne copie les données de carte.

**Step 3: reprendre avec Aside**

Aside vérifie :

- paiement accepté ;
- abonnement actif ;
- SKU Basic ;
- compte Artifact Signing créé.

**Step 4: consigner**

Consigner uniquement la date, le SKU et le statut. Ne pas consigner la référence de carte ou de
transaction.

### Task 10: Finaliser l’identité et le certificat avec Aside

**Files:** aucun

**Step 1: soumettre l’identité**

Aside remplit l’identité organisation avec :

- raison sociale exacte ;
- site `skillreg.dev` ou domaine Kairia approprié ;
- email professionnel ;
- adresse légale ;
- document officiel ou D-U-N-S si disponible.

**Step 2: vérifier les emails**

Aside ouvre la boîte connectée et suit les liens de vérification.

**Step 3: attendre sans bloquer**

Si Microsoft place la validation en attente :

- fermer proprement la session Aside ;
- noter le statut ;
- reprendre plus tard avec une nouvelle session Aside ;
- ne pas redemander d’action à Axel.

**Step 4: créer le profil**

Une fois l’identité validée :

- créer `skillreg-public-trust` ;
- sélectionner `Public Trust` ;
- vérifier le sujet du certificat ;
- copier uniquement le sujet public et le statut.

### Task 11: Configurer l’identité GitHub OIDC avec Aside

**Files:** aucun

**Step 1: créer l’application Entra**

Créer `skillreg-github-release-signing`.

**Step 2: créer le credential fédéré**

Limiter la confiance au dépôt :

```text
Tontoon7/skillreg-local
```

et à l’environnement :

```text
release-signing
```

Le sujet OIDC doit être suffisamment restrictif pour qu’une autre branche ou un autre dépôt ne
puisse pas signer.

**Step 3: attribuer le rôle minimal**

Attribuer `Artifact Signing Certificate Profile Signer` au scope du profil, pas à toute la
subscription.

**Step 4: créer l’environnement GitHub avec Aside**

Dans GitHub Settings :

- créer `release-signing` ;
- ajouter les variables non secrètes ;
- limiter l’environnement au workflow/tag de release ;
- ne pas ajouter d’approbation humaine obligatoire, afin qu’Axel ne soit pas sollicité après le
  paiement ;
- vérifier les protections de tag et de branche.

**Step 5: confirmer**

Avec `aside repl`, vérifier l’existence des variables sans afficher leur éventuelle valeur complète
dans les captures.

### Task 12: Configurer les secrets Linux avec Aside

**Files:** aucun

**Step 1: ouvrir GitHub Settings dans Aside**

Utiliser la session connectée existante.

**Step 2: ajouter les secrets**

Ajouter directement :

```text
LINUX_GPG_PRIVATE_KEY_B64
LINUX_GPG_PASSPHRASE
```

Ne jamais coller les valeurs dans un terminal journalisé ou un message.

**Step 3: ajouter les variables publiques**

Ajouter :

```text
LINUX_GPG_KEY_ID
LINUX_GPG_FINGERPRINT
```

**Step 4: vérifier**

Confirmer seulement les noms et la date de mise à jour des secrets.

### Task 13: Exécuter le preflight Windows

**Files:**

- Modify if needed: `src-tauri/scripts/sign-windows.ps1`
- Modify if needed: `src-tauri/tauri.windows.conf.json`
- Modify: `.github/workflows/release.yml`

**Step 1: ajouter un déclenchement manuel sans publication**

Ajouter temporairement ou durablement :

```yaml
workflow_dispatch:
  inputs:
    signing_preflight:
      type: boolean
      default: true
```

Le mode preflight doit construire et signer sans créer de release.

**Step 2: lancer le workflow**

Run via GitHub Actions, sans tag public.

Expected :

- OIDC accepté ;
- compte et profil accessibles ;
- Tauri appelle le signer pour le binaire et les installateurs ;
- aucun client secret requis.

**Step 3: vérifier les artefacts**

Exécuter `verify-windows-signatures.ps1`.

Expected :

- tous les statuts `Valid` ;
- sujet Kairia ;
- timestamp présent.

**Step 4: tester le binaire installé**

Installer le NSIS sur une VM Windows 11 propre puis vérifier le binaire dans son dossier
d’installation.

**Step 5: traiter le repli uniquement si nécessaire**

Si l’OIDC ne fonctionne pas dans `signCommand`, documenter l’erreur et activer le repli secret
90 jours via Aside. Ne pas modifier le plan de sécurité sans preuve.

### Task 14: Exécuter le preflight Linux

**Files:**

- Modify if needed: `.github/workflows/release.yml`
- Modify if needed: `scripts/verify-linux-signatures.sh`

**Step 1: lancer un build Linux sans publication**

Expected :

- AppImage ;
- RPM ;
- DEB ;
- archive updater et `.sig` ;
- AppImage signé ;
- RPM signé.

**Step 2: générer le manifeste**

Créer `SHA256SUMS` et `SHA256SUMS.asc`.

**Step 3: vérifier**

Run :

```bash
bash scripts/verify-linux-signatures.sh artifacts
```

Expected : `PASS`.

**Step 4: tester sur un environnement propre**

Vérifier au minimum :

- Ubuntu LTS compatible avec le runner de build ;
- installation du `.deb` ;
- exécution de l’AppImage ;
- installation ou inspection du RPM sur une distribution compatible ;
- update Tauri depuis la version stable précédente.

### Task 15: Ajouter attestations et gate de publication

**Files:**

- Modify: `.github/workflows/release.yml`

**Step 1: attester les artefacts**

Après vérification, utiliser `actions/attest` pour les artefacts publics.

**Step 2: vérifier le contenu de `latest.json`**

Échouer si l’une de ces plateformes manque :

```text
darwin-aarch64
darwin-x86_64
linux-x86_64
windows-x86_64
```

**Step 3: vérifier les signatures avant upload**

Le job `publish` doit recevoir un petit manifeste de résultat signé ou des marqueurs produits par
chaque job de build. Il ne doit pas inférer la réussite à partir de la simple présence du fichier.

**Step 4: publier les preuves**

Uploader :

```text
SHA256SUMS
SHA256SUMS.asc
skillreg-release-signing-key.asc
latest.json
```

**Step 5: rendre la release publique**

Après toutes les gates :

```bash
gh release edit "$TAG" --repo "$GH_REPO" --draft=false
```

Si une gate échoue, conserver le brouillon et ne jamais publier partiellement.

### Task 16: Publier la clé Linux sur le site

**Files in `skillreg-website`:**

- Create: `public/skillreg-release-signing-key.asc`
- Modify: `src/app/download/page.tsx`
- Modify: relevant download documentation under `src/content/docs`
- Modify: `src/app/llms.txt/route.ts` if download verification is described there

**Step 1: vérifier le worktree du site**

```bash
git -C ../skillreg-website status --short
```

**Step 2: ajouter la clé publique**

Ne jamais ajouter de clé privée ou certificat de révocation.

**Step 3: afficher l’empreinte**

La page de téléchargement indique :

- empreinte complète ;
- lien vers la clé ;
- lien vers `SHA256SUMS` ;
- commande de vérification ;
- différence entre signature Linux et signature de l’updater.

**Step 4: tester**

```bash
cd ../skillreg-website
npm run lint
npm run build
```

Expected : `PASS`.

### Task 17: Mettre à jour les documents de release

**Files:**

- Modify: `docs/plans/2026-07-29-managed-skills-release-checklist.md`
- Create: `docs/release-signing-runbook.md`
- Modify: `ROADMAP.md`
- Modify: `DEV-PLAN.md`

**Step 1: écrire le runbook**

Inclure :

- ressources Azure publiques ;
- noms des variables et secrets, jamais leurs valeurs ;
- procédure Aside ;
- paiement ;
- vérification Windows/Linux ;
- rotation et révocation ;
- panne Azure ;
- compromission GPG ;
- clé Tauri perdue ;
- rollback d’une release ;
- renouvellement et test trimestriel.

**Step 2: mettre à jour la checklist**

Ne cocher une gate qu’avec une preuve réelle.

**Step 3: mettre à jour la roadmap**

Marquer séparément :

- signature Windows ;
- signature Linux ;
- update E2E Windows ;
- update E2E Linux ;
- dépôt APT/RPM futur.

### Task 18: Première release signée

**Files:** aucun changement obligatoire avant exécution

**Step 1: refaire toutes les gates locales**

```bash
pnpm install --frozen-lockfile
pnpm format:check
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
pnpm release:check-signing
bash scripts/check-release-notarization.sh
```

Expected : `PASS`.

**Step 2: vérifier les versions**

Les versions doivent correspondre dans :

- `package.json` ;
- `src-tauri/tauri.conf.json` ;
- `src-tauri/Cargo.toml` si applicable ;
- tag Git.

**Step 3: déclencher la release**

Créer le tag uniquement après autorisation de finalisation conformément au workflow Git du projet.

**Step 4: surveiller**

Suivre chaque job, sans imprimer de secrets. Une plateforme rouge bloque la publication.

**Step 5: installer**

Tester les téléchargements publics sur :

- Windows 11 propre ;
- Ubuntu LTS propre ;
- une distribution RPM compatible.

**Step 6: tester l’updater**

Depuis la dernière version stable :

- chercher la mise à jour ;
- télécharger ;
- vérifier ;
- installer ;
- relancer ;
- confirmer la nouvelle version ;
- vérifier la signature du binaire installé.

**Step 7: clôturer**

Consigner :

- tag ;
- commit ;
- empreintes SHA-256 ;
- sujet Authenticode ;
- empreinte GPG ;
- résultats des installations ;
- résultat des updates ;
- absence de secret dans les logs.

## 9. Scénarios d’échec et rollback

### Azure indisponible

- le build Windows échoue ;
- aucun artefact Windows unsigned n’est collecté ;
- la release reste en brouillon ;
- ne jamais contourner la gate avec `--no-sign`.

### Identité Azure refusée

- Aside collecte le motif public ;
- corrige les informations avec les justificatifs disponibles ;
- ne recrée pas plusieurs comptes payants ;
- ne sollicite Axel que si Microsoft exige une déclaration personnelle non délégable.

### OIDC refusé

- vérifier le sujet fédéré, l’audience, le tenant et le scope du rôle ;
- vérifier que le workflow utilise l’environnement `release-signing` ;
- ne pas élargir le rôle à la subscription par facilité ;
- n’activer le secret client temporaire qu’après diagnostic documenté.

### Clé GPG invalide

- arrêter le job Linux ;
- supprimer le trousseau temporaire ;
- ne pas publier de checksum non signé ;
- vérifier l’empreinte attendue avant toute nouvelle tentative.

### Compromission GPG

- publier le certificat de révocation ;
- révoquer la sous-clé ;
- générer une nouvelle sous-clé ;
- mettre à jour GitHub via Aside ;
- publier la nouvelle clé et son empreinte ;
- conserver les anciens checksums et preuves pour l’historique.

### Clé Tauri perdue

- ne pas régénérer silencieusement une nouvelle paire ;
- les clients installés ne pourraient pas valider les updates ;
- déclencher le plan de récupération depuis la sauvegarde ;
- si la récupération échoue, préparer une migration explicite via une release signée par
  l’ancienne clé encore installable.

### Signature valide mais SmartScreen présent

- ne pas considérer cela comme un échec cryptographique ;
- vérifier le sujet, le timestamp et la chaîne ;
- soumettre le fichier à Microsoft si nécessaire ;
- laisser la réputation se construire ;
- ne pas acheter un EV uniquement pour contourner cet état.

## 10. Hors périmètre

- Microsoft Store/MSIX ;
- Snap Store ou Flathub ;
- dépôt APT public ;
- dépôt YUM/DNF public ;
- Windows ARM64 ;
- Linux ARM64 ;
- rotation de la clé Tauri ;
- changement du modèle d’auto-update ;
- signature macOS, déjà gérée séparément ;
- collecte de télémétrie liée aux installations.

## 11. Phase ultérieure recommandée

Après stabilisation de deux releases signées :

1. dépôt APT signé pour Ubuntu/Debian ;
2. dépôt RPM signé pour Fedora/RHEL ;
3. éventuelle publication Microsoft Store ;
4. builds Windows ARM64 et Linux ARM64 ;
5. test automatisé trimestriel de rotation/révocation ;
6. SBOM signé par release.

## 12. Sources officielles

- Tauri Windows signing:
  `https://v2.tauri.app/distribute/sign/windows/`
- Tauri Linux signing:
  `https://v2.tauri.app/distribute/sign/linux/`
- Tauri updater:
  `https://v2.tauri.app/plugin/updater/`
- Azure Artifact Signing:
  `https://learn.microsoft.com/azure/artifact-signing/overview`
- Azure GitHub OIDC:
  `https://learn.microsoft.com/azure/developer/github/connect-from-azure-openid-connect`
- Artifact Signing GitHub Action:
  `https://github.com/Azure/artifact-signing-action`
- Debian package trust:
  `https://www.debian.org/doc/manuals/securing-debian-manual/deb-pack-sign.en.html`
- RPM signing:
  `https://rpm.org/docs/6.1.x/man/rpmsign.1`
- GitHub artifact attestations:
  `https://docs.github.com/actions/how-tos/secure-your-work/use-artifact-attestations/`

## 13. Définition de terminé

Le lot n’est terminé que si :

- Azure Basic est actif sous l’identité juridique attendue ;
- Aside a réalisé toutes les mutations de comptes ;
- Axel n’a été sollicité qu’au paiement, hors contrôle légal techniquement non délégable ;
- OIDC signe sans secret Windows permanent, ou le repli temporaire est explicitement documenté ;
- Windows, AppImage, RPM, checksums et updater passent leurs vérifications ;
- les preuves sont publiées sans données sensibles ;
- une mise à jour réelle depuis la version stable précédente réussit sur Windows et Linux ;
- la release publique ne contient aucun artefact unsigned ;
- le runbook et la checklist reflètent les résultats réellement observés.
