---
id: installing-stackql-deploy
title: Install
hide_title: false
hide_table_of_contents: false
description: Installation options for StackQL Deploy across all platforms.
tags: []
draft: false
unlisted: false
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Install `stackql-deploy`

`stackql-deploy` is distributed as a standalone, statically compiled binary with no runtime dependencies. Choose your preferred installation method below.

## Package Managers

<Tabs>
<TabItem value="homebrew" label="Homebrew (macOS/Linux)">

```bash
brew tap stackql/tap
brew install stackql-deploy
```

To upgrade to the latest version:

```bash
brew upgrade stackql-deploy
```

</TabItem>
<TabItem value="chocolatey" label="Chocolatey (Windows)">

```powershell
choco install stackql-deploy
```

To upgrade to the latest version:

```powershell
choco upgrade stackql-deploy
```

</TabItem>
</Tabs>

## Platform-Specific Installers

### macOS

Download the latest `.pkg` installer from the [GitHub Releases](https://github.com/stackql-labs/stackql-deploy-rs/releases) page. The installer supports both Intel and Apple Silicon Macs.

### Windows

Download the latest `.msi` installer from the [GitHub Releases](https://github.com/stackql-labs/stackql-deploy-rs/releases) page.

### Linux

Download the latest archive for your architecture from the [GitHub Releases](https://github.com/stackql-labs/stackql-deploy-rs/releases) page:

```bash
# x86_64
curl -L https://github.com/stackql-labs/stackql-deploy-rs/releases/latest/download/stackql-deploy-linux-x86_64.tar.gz | tar xz
sudo mv stackql-deploy /usr/local/bin/

# aarch64
curl -L https://github.com/stackql-labs/stackql-deploy-rs/releases/latest/download/stackql-deploy-linux-aarch64.tar.gz | tar xz
sudo mv stackql-deploy /usr/local/bin/
```

## GitHub Actions

For CI/CD workflows, see the [GitHub Actions](/github-actions) documentation for using `stackql-deploy` in your pipelines.

## Verify Installation

After installation, verify that `stackql-deploy` is available:

```bash
stackql-deploy info
```

This will display the installed version and environment details.
