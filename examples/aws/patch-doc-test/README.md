# Patch Document Test (Cloud Control UPDATE)

Demonstrates the AWS Cloud Control API update workflow using `PatchDocument` with an S3 bucket. The stack deploys a bucket with versioning enabled, and subsequent builds detect configuration drift and apply updates via the Cloud Control `UpdateResource` action.

## What This Tests

1. **Create** - Deploy an S3 bucket with `VersioningConfiguration: Enabled`
2. **Update** - Change the versioning config in the manifest (e.g. to `Suspended`), re-run build
3. **Statecheck** detects the drift (current state != desired state)
4. **PatchDocument** is generated via the `generate_patch_document` filter and applied
5. **Post-update statecheck** confirms the update was applied

## The PatchDocument Pattern

The `update` anchor uses the `generate_patch_document` Tera filter to transform manifest property values into a Cloud Control API `PatchDocument`:

```sql
/*+ update */
UPDATE awscc.s3.buckets
SET PatchDocument = string('{{ {
    "VersioningConfiguration": bucket1_versioning_config,
    "Tags": bucket1_tags
    } | generate_patch_document }}')
WHERE Identifier = '{{ bucket1_name }}'
AND region = '{{ region }}';
```

This generates a JSON Patch array like:

```json
[
  {"op": "add", "path": "/VersioningConfiguration", "value": {"Status": "Suspended"}},
  {"op": "add", "path": "/Tags", "value": [...]}
]
```

## Prerequisites

- `stackql-deploy` installed ([releases](https://github.com/stackql/stackql-deploy-rs/releases))
- AWS credentials set:

  ```bash
  export AWS_ACCESS_KEY_ID=your_access_key
  export AWS_SECRET_ACCESS_KEY=your_secret_key
  export AWS_REGION=us-east-1
  ```

## Usage

### Deploy (create bucket with versioning Enabled)

```bash
stackql-deploy build examples/aws/patch-doc-test dev
```

### Update versioning config

Edit `stackql_manifest.yml` and change:

```yaml
- name: bucket1_versioning_config
  value:
    Status: Suspended  # was: Enabled
```

Then re-deploy:

```bash
stackql-deploy build examples/aws/patch-doc-test dev
```

The build will detect the drift, generate a PatchDocument, and apply the update.

### Test

```bash
stackql-deploy test examples/aws/patch-doc-test dev
```

### Teardown

```bash
stackql-deploy teardown examples/aws/patch-doc-test dev
```

### Debug mode

```bash
stackql-deploy build examples/aws/patch-doc-test dev --log-level debug
```
