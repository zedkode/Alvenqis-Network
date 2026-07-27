# Cum creezi un tag si un release Alvenqis

## Instalare

Din folderul extras ruleaza:

```powershell
.\install-workflows.ps1 -RepoPath "D:\Blockchain-Core\Alvenqis_Network"
```

Installerul copiaza workflow-urile in `.github/workflows/` si managerul interactiv in `scripts/release/`.

Dupa instalare, verifica modificarile, apoi fa commit si push:

```powershell
cd D:\Blockchain-Core\Alvenqis_Network
git status
git add .github/workflows scripts/release
git commit -m "ci: independent releases and local artifact publishing"
git push
```

## Pornire manager interactiv

Din root-ul repository-ului:

```powershell
.\scripts\release\alvenqis-release.cmd
```

Alege optiunea `1` pentru urmatorul candidate tag.

Managerul verifica automat folderul:

```text
D:\Blockchain-Core\Alvenqis_Network\release-artifacts
```

Implicit considera „recente” fisierele modificate in ultimele 24 de ore. Poti introduce alt interval intre 1 si 720 de ore.

Daca gaseste fisiere eligibile, le afiseaza cu nume, marime si data. Poti:

1. urca toate fisierele recente;
2. selecta manual doar anumite fisiere;
3. ignora fisierele locale si lasa GitHub Actions sa faca build.

## Ce se intampla cand folosesti artefacte locale

Scriptul:

- creeaza si publica tag-ul, de exemplu `v1.0.0-candidate.3`;
- creeaza sau reutilizeaza GitHub prerelease-ul;
- urca fisierele selectate cu `--clobber`;
- genereaza automat `SHA256SUMS-LOCAL.txt`;
- marcheaza in tag platformele deja construite local.

Marcajele sunt independente:

- `[local-windows]` opreste doar build-ul Windows;
- `[local-linux]` opreste doar build-ul Linux;
- `[local-vps]` opreste doar build-ul VPS.

Exemplu: daca in `release-artifacts` exista doar un `.exe` recent, Windows nu se reconstruieste, dar Linux, VPS si Quality continua normal.

## Daca nu exista artefacte recente

Push-ul tag-ului porneste automat si independent:

- Candidate Windows Release;
- Candidate Linux Release;
- Candidate VPS Release;
- Candidate Quality Checks.

Primul workflow de platforma care termina cu succes creeaza prerelease-ul. Celelalte adauga fisiere in acelasi release.

## Urcare intr-un tag deja existent

Ruleaza managerul si alege optiunea `4`:

```text
Urca artefacte locale intr-un tag existent
```

Selectezi tag-ul, intervalul de timp si fisierele. Duplicate dupa nume sunt inlocuite.

## Daca vrei sa reconstruiesti fortat o platforma

Alege optiunea `3`, selecteaza tag-ul si platforma. Managerul trimite automat `force_build=true`, deci platforma este reconstruita chiar daca tag-ul are un marcaj local.

## Cerinte

- Git instalat si disponibil in `PATH`;
- repository-ul trebuie sa aiba remote-ul `origin`;
- GitHub CLI instalat si autentificat pentru upload local si relansari:

```powershell
gh auth login
```
