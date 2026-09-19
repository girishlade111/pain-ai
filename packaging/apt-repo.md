# Debian / Ubuntu APT Repository Guide for pain ai (`lsc`)

This document outlines the submit-ready configuration and hosting protocol for the official **pain ai** APT repository.

---

## 1. User Installation Instructions

To install `pain-ai` on Ubuntu 22.04 LTS or Debian 12+:

```bash
# 1. Install prerequisites
sudo apt-get update && sudo apt-get install -y curl gpg apt-transport-https

# 2. Download the LadeStack package signing key
curl -fsSL https://apt.ladestack.in/KEY.gpg | sudo gpg --dearmor -o /etc/apt/keyrings/ladestack-archive-keyring.gpg

# 3. Add the pain ai repository source
echo "deb [signed-by=/etc/apt/keyrings/ladestack-archive-keyring.gpg] https://apt.ladestack.in/ stable main" | sudo tee /etc/apt/sources.list.d/pain-ai.list

# 4. Update index and install
sudo apt-get update
sudo apt-get install -y pain-ai

# 5. Run diagnostic validation
lsc doctor
```

---

## 2. Repository Infrastructure Specification

### Directory Structure
```
apt.ladestack.in/
├── KEY.gpg                             # Exported GPG public key
├── dists/
│   └── stable/
│       ├── Release                     # Plaintext release metadata
│       ├── Release.gpg                 # Detached signature
│       ├── InRelease                   # Clearsigned release metadata
│       └── main/
│           └── binary-amd64/
│               ├── Packages            # Index of Debian packages
│               ├── Packages.gz         # Gzip-compressed index
│               └── Release
└── pool/
    └── main/
        └── p/
            └── pain-ai/
                └── pain-ai_1.0.0_amd64.deb
```

### Package Dependencies (`control` metadata)
- Package: `pain-ai`
- Version: `1.0.0`
- Section: `utils`
- Priority: `optional`
- Architecture: `amd64`
- Depends: `libc6 (>= 2.34), libwebkit2gtk-4.1-0 (>= 2.36), libgtk-3-0 (>= 3.24), at-spi2-core, libglib2.0-0`
- Maintainer: `Girish Lade <girish@ladestack.in>`
- Description: `Local-first, cross-platform desktop AI agent (LadeStack Companion)`

---

## 3. Repository Signing Protocol

Repository index files are generated using `reprepro` or `dpkg-scanpackages`:

```bash
# Generate Packages index
dpkg-scanpackages --multiversion pool/main > dists/stable/main/binary-amd64/Packages
gzip -k -f dists/stable/main/binary-amd64/Packages

# Generate Release and Sign
cd dists/stable
apt-ftparchive release . > Release
gpg --default-key "release@ladestack.in" -abs -o Release.gpg Release
gpg --default-key "release@ladestack.in" --clearsign -o InRelease Release
```
