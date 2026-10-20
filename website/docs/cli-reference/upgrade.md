---
title: upgrade
hide_title: true
hide_table_of_contents: false
keywords:
  - stackql
  - stackql-deploy
  - infrastructure-as-code
  - configuration-as-data
tags:
  - stackql
  - stackql-deploy
  - infrastructure-as-code
  - configuration-as-data  
description: Documentation for the upgrade command in StackQL Deploy
image: "/img/stackql-cover.png"
---

# <span className="docFieldHeading">`upgrade`</span>

Command used to upgrade `stackql-deploy` and the `stackql` binary to the latest versions.

* * *

## Syntax

<code>stackql-deploy <span className="docFieldHeading">upgrade</span></code>

* * *

## Description

The `upgrade` command automates the process of upgrading the `stackql` binary to the latest available version. This ensures that your environment is up-to-date with the latest features, improvements, and security patches.

To upgrade the `stackql-deploy` binary itself, use your package manager:

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

<Tabs>
<TabItem value="macos" label="macOS (Homebrew)">

```bash
brew upgrade stackql-deploy
```

</TabItem>
<TabItem value="windows" label="Windows (Chocolatey)">

```powershell
choco upgrade stackql-deploy
```

</TabItem>
<TabItem value="linux" label="Linux">

Download the latest binary from the [GitHub Releases](https://github.com/stackql-labs/stackql-deploy-rs/releases) page.

</TabItem>
</Tabs>

## Examples

### Upgrade the `stackql` binary to the latest version

```bash
stackql-deploy upgrade
```

outputs...

```plaintext
upgrading stackql binary, current version v0.5.708...
stackql binary upgraded to v0.6.002.
```

If the `stackql` binary is already up-to-date, the command will notify you accordingly.
