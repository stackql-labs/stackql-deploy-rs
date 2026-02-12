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
stackql-deploy version: 0.1.0
stackql version       : v0.6.002
stackql binary path   : /usr/local/bin/stackql
platform              : Linux x86_64

installed providers:  :
aws                   : v24.07.00246
azure                 : v24.06.00242
google                : v24.06.00236
```
