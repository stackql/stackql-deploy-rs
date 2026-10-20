---
title: info
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
description: Documentation for the info command in StackQL Deploy
image: "/img/stackql-cover.png"
---

# <span className="docFieldHeading">`info`</span>

Command used to display version information and environment details for the StackQL Deploy program and its dependencies.

* * *

## Syntax

<code>stackql-deploy <span className="docFieldHeading">info</span> [FLAGS]</code>

* * *

* * *

## Description

The `info` command provides detailed information about the StackQL Deploy environment, including the versions of `stackql-deploy` and the `stackql` binary, as well as platform information. Additionally, the command lists all installed providers and their versions.

## Examples

### Display version information

Display the version information of the `stackql-deploy` tool and the installed providers:

```bash
stackql-deploy info
```
outputs...

```plaintext
┌────────────────────────────────┐
│ Getting program information... │
└────────────────────────────────┘
stackql-deploy CLI
  Version: 0.1.0

StackQL Library
  Version: v0.10.383
  SHA: 3374f33
  Platform: Linux
  Binary Path: /mnt/c/LocalGitRepos/stackql/stackql-deploy-rs/stackql

Local StackQL Servers
  None

Installed Providers
  awscc v26.02.00373
  databricks_account v26.02.00371
  databricks_workspace v26.02.00371
  github v25.07.00320
```
