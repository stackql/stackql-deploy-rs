# stackql-deploy

[![Crates.io](https://img.shields.io/crates/v/stackql-deploy.svg)](https://crates.io/crates/stackql-deploy)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**stackql-deploy** is an infrastructure-as-code framework for declarative cloud resource management using [StackQL](https://stackql.io). Define your cloud resources once using StackQL queries and YAML manifests, then `build`, `test`, and `teardown` across any environment.

> This is the Rust rewrite (v2.x). The original Python package (`stackql-deploy` on PyPI, v1.x) is archived — see the [Python package changelog](https://github.com/stackql/stackql-deploy/blob/main/CHANGELOG.md) for prior release history.

---

## Install

### Via `cargo`

```sh
cargo install stackql-deploy
```

### Direct binary download

Pre-built binaries for Linux (x86_64 / ARM64), macOS (Apple Silicon / Intel), and Windows (x86_64) are available on the [GitHub Releases](https://github.com/stackql/stackql-deploy/releases) page.

**Linux / macOS:**

```sh
# Replace <version> and <target> as appropriate, e.g. 2.0.0 and linux-x86_64
curl -L https://github.com/stackql/stackql-deploy/releases/download/v<version>/stackql-deploy-<target>.tar.gz | tar xz
chmod +x stackql-deploy
sudo mv stackql-deploy /usr/local/bin/
```

**Windows:**

Download the `.zip` from the releases page and add the extracted binary to your `PATH`.

---

## Quick start

### 1. Initialise a new project

```sh
# Using a built-in provider template
stackql-deploy init my-stack --provider aws

# Using a template from the template hub
stackql-deploy init my-stack --template google/starter
```

### 2. Build (deploy) your stack

```sh
stackql-deploy build my-stack dev
```

### 3. Test your stack

```sh
stackql-deploy test my-stack dev
```

### 4. Tear down your stack

```sh
stackql-deploy teardown my-stack dev
```

### Other commands

```sh
# Show version / provider info
stackql-deploy info

# Interactive StackQL shell
stackql-deploy shell

# Update the embedded StackQL binary
stackql-deploy upgrade

# Preview what build would do (no changes applied)
stackql-deploy build my-stack dev --dry-run
```

---

## Project structure

A `stackql-deploy` project consists of a manifest file and one or more StackQL query files:

```
my-stack/
├── stackql_manifest.yml   # Declares resources, providers, and environment config
└── resources/
    └── my_bucket.iql      # StackQL queries for create/exists/state checks
```

See the [documentation site](https://stackql.io/docs/stackql-deploy) for the full manifest reference and query file format.

---

## Supported providers

stackql-deploy works with any provider supported by StackQL, including AWS, Google Cloud, Azure, Databricks, Snowflake, and more. See [registry.stackql.io](https://registry.stackql.io) for the full provider list.

---

## License

MIT — see [LICENSE](LICENSE) for details.
