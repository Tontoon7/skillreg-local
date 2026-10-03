# Dashboard compact et import des skills locales — Plan d’implémentation

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Corriger le dashboard à la largeur minimale et permettre à tout utilisateur connecté
d’importer ses skills locales dans le stockage canonique SkillReg sans publication.

**Architecture:** Le manifeste géré v2 distingue les origines `registry` et `local`. Un nouveau
service Rust inspecte, copie et lie transactionnellement les dossiers locaux. Le frontend expose
l’aperçu et l’action depuis les Réglages, puis rafraîchit le store géré. Le dashboard est rendu
réductible par des contraintes flex explicites et des contrôles qui se replient.

**Tech Stack:** Rust/Tauri v2, React 19, TypeScript, Zustand, Vitest, Testing Library, Cargo tests,
Tailwind CSS v4.

**Statut :** terminé et validé localement le 2026-07-29.

---

## Task 1: Verrouiller le défaut responsive

**Files:**

- Modify: `src/components/layout/__tests__/AppShell.test.tsx`
- Modify: `src/pages/__tests__/Dashboard.test.tsx`
- Modify: `src/components/layout/AppShell.tsx`
- Modify: `src/pages/Dashboard.tsx`
- Modify: `src/components/dashboard/AutoUpdateControl.tsx`
- Modify: `src/components/dashboard/RequiredActions.tsx`

1. Ajouter un test qui exige une zone principale réductible (`min-w-0`) et une sidebar
   non réductible.
2. Ajouter un test qui exige que les groupes de contrôles du dashboard puissent occuper toute la
   largeur compacte et se replier.
3. Exécuter les tests ciblés et constater l’échec.
4. Ajouter uniquement les contraintes flex et responsive nécessaires.
5. Relancer les tests ciblés.

## Task 2: Distinguer les skills locales des skills du registre

**Files:**

- Modify: `src-tauri/src/managed_skills/mod.rs`
- Modify: `src-tauri/src/managed_skills/manifest.rs`
- Modify: `src-tauri/src/managed_skills/service.rs`
- Modify: `src-tauri/src/commands/managed_skills.rs`
- Modify: `src/lib/types.ts`
- Modify: tests Rust et TypeScript concernés

1. Ajouter des tests de désérialisation prouvant que l’absence d’origine vaut `registry`.
2. Ajouter un test de décision de mise à jour prouvant qu’une origine locale est ignorée avant
   toute résolution distante.
3. Exécuter les tests ciblés et constater l’échec.
4. Ajouter `ManagedSkillOrigin` avec valeur par défaut rétrocompatible.
5. Sérialiser l’origine dans le DTO et le type TypeScript.
6. Exclure les entrées locales du cycle de mise à jour.
7. Relancer les tests ciblés.

## Task 3: Construire l’aperçu local en lecture seule

**Files:**

- Create: `src-tauri/src/managed_skills/local_import.rs`
- Modify: `src-tauri/src/managed_skills/mod.rs`
- Create: `src-tauri/tests/local_skill_import.rs`
- Reuse: `src-tauri/tests/support/*`

1. Écrire des fixtures pour un dossier valide, deux copies identiques, deux copies divergentes,
   un lien vers une candidate, un lien externe et une skill déjà gérée.
2. Écrire les assertions sur les classifications et compteurs.
3. Vérifier que l’aperçu n’a modifié aucun inode ni contenu.
4. Exécuter le test et constater l’échec.
5. Implémenter le scan des seuls répertoires utilisateur reconnus, sans suivre les liens
   arbitraires.
6. Grouper par nom et hash, puis produire un DTO sans chemin.
7. Relancer le test ciblé.

## Task 4: Implémenter l’import transactionnel

**Files:**

- Modify: `src-tauri/src/managed_skills/local_import.rs`
- Modify: `src-tauri/tests/local_skill_import.rs`

1. Écrire un test de succès : copie canonique, manifeste local, liens vérifiés, doublons réunis.
2. Écrire un test de conflit : aucune mutation.
3. Écrire un test d’échec après déplacement : restauration des sources et absence d’entrée
   manifeste.
4. Écrire un test d’idempotence.
5. Exécuter les tests et constater l’échec.
6. Implémenter staging, copie sûre, vérification de hash, sauvegardes temporaires, bindings,
   écriture atomique et rollback.
7. Relancer les tests ciblés.

## Task 5: Exposer les commandes Tauri et les contrats frontend

**Files:**

- Create: `src-tauri/src/commands/local_import.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/api.ts`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/__tests__/managed-api.test.ts`

1. Écrire le test frontend qui attend `preview_local_skills_import` sans argument sensible et
   `run_local_skills_import` avec confirmation explicite.
2. Exécuter le test et constater l’échec.
3. Ajouter les commandes Rust ; l’organisation active vient de la configuration locale.
4. Ajouter les wrappers typés.
5. Relancer les tests ciblés et `cargo check`.

## Task 6: Rendre l’option accessible dans les Réglages

**Files:**

- Create: `src/components/settings/LocalSkillImportPanel.tsx`
- Create: `src/components/settings/__tests__/LocalSkillImportPanel.test.tsx`
- Modify: `src/pages/Settings.tsx`

1. Tester les états aperçu, vide, conflit laissé intact, progression, succès et erreur.
2. Tester qu’un clic d’import appelle la commande confirmée puis rafraîchit le store.
3. Exécuter les tests et constater l’échec.
4. Implémenter un panneau métier sans agent, scope, chemin ou version.
5. Insérer le panneau dans la section Application des Réglages pour tous les utilisateurs
   connectés.
6. Relancer les tests ciblés.

## Task 7: Valider le poste réel et effectuer la bascule

**Files:**

- Modify: `ROADMAP.md`
- Modify: `docs/plans/2026-07-29-managed-skills-release-checklist.md`

1. Exécuter `pnpm test`, `pnpm format:check`, `pnpm build`.
2. Exécuter `cargo test --manifest-path src-tauri/Cargo.toml`.
3. Exécuter `cargo check --manifest-path src-tauri/Cargo.toml`.
4. Lancer l’application native et vérifier le dashboard à 900×600.
5. Ouvrir les Réglages et capturer l’aperçu réel.
6. Vérifier que les liens externes sont classés « laissés intacts ».
7. Confirmer l’import réel depuis l’interface.
8. Vérifier le manifeste géré sans afficher de token ni contenu de skill.
9. Vérifier les liens et ouvrir le dashboard : une ligne canonique par skill.
10. Mettre à jour la roadmap et la checklist avec les résultats exacts.

## Résultats

- Les tests ont été écrits en échec avant les changements de logique et d’interface.
- Le dashboard tient dans la fenêtre minimale Tauri de 900×600.
- L’import reconnaît aussi les descriptions YAML multilignes et les anciens noms d’affichage.
- La bascule réelle a centralisé 47 skills et créé 141 bindings vérifiés.
- Le lien externe `decktype` est resté inchangé.
- Les skills d’origine `local` sont exclues de toute résolution ou mise à jour distante.
- L’entrée de barre de menus recrée la fenêtre principale si le webview n’existe plus.
