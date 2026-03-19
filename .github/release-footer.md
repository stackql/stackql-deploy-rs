### Download

| Platform | Architecture | Asset |
|----------|--------------|-------|
| Linux    | x86_64       | `stackql-deploy-linux-x86_64.tar.gz` |
| Linux    | arm64        | `stackql-deploy-linux-arm64.tar.gz` |
| macOS    | Universal (Apple Silicon + Intel) | `stackql-deploy-macos-universal.tar.gz` |
| Windows  | x86_64       | `stackql-deploy-windows-x86_64.zip` |

Each archive contains a single binary named `stackql-deploy` (or `stackql-deploy.exe` on Windows). Verify your download with `SHA256SUMS`.

### Install (quick)

**Linux / macOS:**

```sh
curl -L https://get-stackql-deploy.rs -o stackql-deploy.tar.gz && tar xz stackql-deploy.tar.gz
```

**Windows (PowerShell):**

```powershell
Invoke-WebRequest -Uri https://get-stackql-deploy.rs -OutFile stackql-deploy.zip
Expand-Archive stackql-deploy.zip -DestinationPath .
```

**cargo:**

```sh
cargo install stackql-deploy
```

---

Full documentation: [stackql-deploy.io](https://stackql-deploy.io) - Source: [github.com/stackql/stackql-deploy](https://github.com/stackql/stackql-deploy)
