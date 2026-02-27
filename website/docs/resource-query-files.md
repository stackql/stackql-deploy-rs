---
id: resource-query-files
title: Resource Query Files
hide_title: false
hide_table_of_contents: false
description: A quick overview of how to get started with StackQL Deploy, including basic concepts and the essential components of a deployment.
tags: []
draft: false
unlisted: false
---

import File from '/src/components/File';

Resource query files include the StackQL query templates to provision, de-provision, update and test resources in your stack.  Resource query files (`.iql` files) are located in the `resources` subdirectory of your project (stack) directory.  The `resources` section of the [`stackql_manifest.yml`](manifest-file) file is used to supply these templates with the correct values for a given environment at deploy time.

:::note

`.iql` is used as a file extension for StackQL query files by convention.  This convention originates from the original name for the StackQL project - InfraQL,  plus `.sql` was taken... 

:::

## Query types

A resource query file (`.iql` file) typically contains multiple StackQL queries.  Seperate queries are demarcated by query anchors (or hints), such as `/*+ create */` or `/*+ update */`.  These hints must be at the beginning of a line in the file, with the resepective query following on the subsequent lines.

:::tip

StackQL follows the ANSI standard for SQL with some custom extensions.  For more information on the StackQL grammar see the [StackQL docs](https://stackql.io/docs).

:::

The types of queries defined in resource files are detailed in the following sections.

### `exists`

`exists` queries are StackQL `SELECT` statements designed to test the existence of a resource by its designated identifier (does not test the desired state).  This is used to determine whether a `create` (`INSERT`) or `update` (`UPDATE`) is required.  A `exists` query needs to return a single row with a single field named `count`.  A `count` value of `1` indicates that the resource exists, a value of `0` would indicate that the resource does not exist.

```sql
/*+ exists */
SELECT COUNT(*) as count FROM google.compute.networks
WHERE name = '{{ vpc_name }}'
AND project = '{{ project }}'
```

`preflight` is an alias for `exists` for backwards compatability, this will be deprecated in a future release.

### `create`

`create` queries are StackQL `INSERT` statements used to create resources that do not exist (in accordance with the `exists` query).

```sql
/*+ create */
INSERT INTO google.compute.networks
(
 project,
 data__name,
 data__autoCreateSubnetworks,
 data__routingConfig
) 
SELECT
'{{ project }}',
'{{ vpc_name }}',
false,
'{"routingMode": "REGIONAL"}'
```

### `createorupdate`

`createorupdate` queries can be StackQL `INSERT` or `UPDATE` statements, these queries are used for idempotent resources (as per the given provider if supported), for example:

```sql
/*+ createorupdate */
INSERT INTO azure.network.virtual_networks(
   virtualNetworkName,
   resourceGroupName, 
   subscriptionId, 
   data__location,
   data__properties,
   data__tags   
)
SELECT
   '{{ vnet_name }}',
   '{{ resource_group_name }}',
   '{{ subscription_id }}',
   '{{ location }}',
   '{"addressSpace": {"addressPrefixes":["{{ vnet_cidr }}"]}}',
   '{{ global_tags }}'
```

:::tip

You can usually identify idempotent resources using the `SHOW METHODS` command for a given resource, the the below example you can see a `create_or_update` method mapped to StackQL `INSERT`:

```plaintext {12}
stackql  >>show methods in azure.network.virtual_networks;
|-------------------------------|--------------------------------|---------|                                                                                                               
|          MethodName           |         RequiredParams         | SQLVerb |                                                                                                               
|-------------------------------|--------------------------------|---------|                                                                                                               
| get                           | resourceGroupName,             | SELECT  |                                                                                                               
|                               | subscriptionId,                |         |                                                                                                               
|                               | virtualNetworkName             |         |                                                                                                               
|-------------------------------|--------------------------------|---------|                                                                                                               
| list                          | resourceGroupName,             | SELECT  |                                                                                                               
|                               | subscriptionId                 |         |                                                                                                               
|-------------------------------|--------------------------------|---------|                                                                                                               
| create_or_update              | resourceGroupName,             | INSERT  |                                                                                                               
|                               | subscriptionId,                |         |                                                                                                               
|                               | virtualNetworkName             |         |                                                                                                               
|-------------------------------|--------------------------------|---------|                                                                                                               
| delete                        | resourceGroupName,             | DELETE  |                                                                                                               
|                               | subscriptionId,                |         |                                                                                                               
|                               | virtualNetworkName             |         |                                                                                                               
|-------------------------------|--------------------------------|---------|                                                                                                               
| check_ip_address_availability | ipAddress, resourceGroupName,  | EXEC    |                                                                                                               
|                               | subscriptionId,                |         |                                                                                                               
|                               | virtualNetworkName             |         |                                                                                                               
|-------------------------------|--------------------------------|---------| 
```

:::

`createorupdate` queries can also be used if a resource is updating the state of a pre-existing resource, for example:

```sql
/*+ createorupdate */
update aws.s3.buckets 
set data__PatchDocument = string('{{ {
    "NotificationConfiguration": transfer_notification_config
    } | generate_patch_document }}') 
WHERE 
region = '{{ region }}' 
AND data__Identifier = '{{ transfer_bucket_name }}';
```

### `delete`

`delete` queries are StackQL `DELETE` statements used to de-provision resources in `teardown` operations.

```sql
/*+ delete */
DELETE FROM google.compute.networks
WHERE network = '{{ vpc_name }}' AND project = '{{ project }}'
```

### `statecheck`

`statecheck` queries are StackQL `SELECT` statements designed to test the desired state of a resource in an environment.  Similar to `exists` queries, `statecheck` queries must return a single row with a single column named `count` with a value of `1` (the resource meets the desired state tests) or `0` (the resource is not in the desired state).  As `statecheck` queries are usually run after `create` or `update` queries, it may be necessary to retry the query to account for the time it takes for the resource to be created or updated by the provider.

```sql
/*+ statecheck, retries=5, retry_delay=10 */
SELECT COUNT(*) as count FROM google.compute.networks
WHERE name = '{{ vpc_name }}'
AND project = '{{ project }}'
AND autoCreateSubnetworks = false
AND JSON_EXTRACT(routingConfig, '$.routingMode') = 'REGIONAL'
```

:::tip

Useful functions for testing the desired state of a resource include [`JSON_EQUAL`](https://stackql.io/docs/language-spec/functions/json/json_equal), [`AWS_POLICY_EQUAL`](https://stackql.io/docs/language-spec/functions/json/aws_policy_equal), [`JSON_EXTRACT`](https://stackql.io/docs/language-spec/functions/json/json_extract) and [`JSON_EACH`](https://stackql.io/docs/language-spec/functions/json/json_equal).

:::

`postdeploy` is an alias for `statecheck` for backwards compatability, this will be deprecated in a future release.

### `exports`

`exports` queries are StackQL `SELECT` statements which export variables, typically used in subsequent (or dependant) resources.  Columns exported in `exports` queries need to be specified in the [`exports` section of the `stackql_manifest.yml` file](manifest-file#resourceexports).

```sql
/*+ exports */
SELECT
'{{ vpc_name }}' as vpc_name,
selfLink as vpc_link
FROM google.compute.networks
WHERE name = '{{ vpc_name }}'
AND project = '{{ project }}'
```

### `callback`

`callback` blocks are optional polling queries that run **after** a `create`, `update`, or `delete` DML statement to track the outcome of a long-running asynchronous operation.  They are only used when the preceding DML statement includes a `RETURNING *` clause that returns a tracking handle from the provider.

The canonical use-case is the AWS Cloud Control API (`awscc`), where every mutation returns a `ProgressEvent` object containing an `OperationStatus` and a `RequestToken` that must be polled until the operation completes.

```sql
/*+ callback:create, retries=10, retry_delay=15, short_circuit_field=ProgressEvent.OperationStatus, short_circuit_value=SUCCESS */
SELECT OperationStatus = 'SUCCESS' as success
FROM awscc.cloudcontrol.resource_request_statuses
WHERE region = '{{ region }}'
AND RequestToken = '{{ callback.ProgressEvent.RequestToken }}'
```

The operation qualifier (`:create`, `:update`, `:delete`) associates the callback with the matching DML anchor.  A plain `/*+ callback */` with no qualifier runs after **any** DML operation on the resource that used `RETURNING *`.

#### Callback options

| Option | Required | Default | Description |
|---|---|---|---|
| `retries` | no | 3 | Maximum number of poll attempts |
| `retry_delay` | no | 5 | Seconds to wait between attempts |
| `short_circuit_field` | no | — | Dot-path into the `RETURNING *` result checked before polling |
| `short_circuit_value` | no | — | Value of `short_circuit_field` that means polling can be skipped |

#### Success condition

The callback query must return a column named **`success`** (or `count`).  The operation is considered complete when the query returns a row where `success` equals `1` or `true`.  If the query returns no rows, or the value is not truthy, the runner waits `retry_delay` seconds and retries.  If all retries are exhausted the run fails with a timeout error.

#### RETURNING * capture and the `callback.*` namespace

When a DML statement includes `RETURNING *` the first row of the provider response is automatically captured and stored in the template context:

- **`callback.{field}`** — shorthand form, available within the current resource's own `.iql` file (overwritten by the next DML with `RETURNING *` on any resource).
- **`{resource_name}.callback.{field}`** — fully-qualified form, available to any downstream resource for the rest of the stack run.

For example, after a `create` on `aws_s3_workspace_bucket` that returns:

```json
{
  "ProgressEvent": {
    "OperationStatus": "SUCCESS",
    "RequestToken": "a0088b9e-db47-4507-b93e-345b77979626"
  }
}
```

The following keys become available:

```
callback.ProgressEvent.OperationStatus          → SUCCESS
callback.ProgressEvent.RequestToken             → a0088b9e-...
aws_s3_workspace_bucket.callback.ProgressEvent.OperationStatus → SUCCESS
aws_s3_workspace_bucket.callback.ProgressEvent.RequestToken    → a0088b9e-...
```

`RETURNING *` without a `callback` block is valid — the result is captured and no polling occurs.

#### Short-circuit

Some providers return the final operation status synchronously in the `RETURNING *` response.  When `short_circuit_field` and `short_circuit_value` are set, the runner checks the named field in the already-captured result **before** making any poll attempt.  If the value matches, the callback is skipped entirely.

#### Scope boundaries

- Callbacks only run during `build` and `teardown`.  The `test` command runs no DML and is unaffected.
- In dry-run mode, neither the `RETURNING *` capture nor the callback query executes.  Intent is logged instead.

#### Known limitations

- There is no mechanism to short-circuit retries on a terminal failure state (e.g. `OperationStatus = 'FAILED'`).  The runner retries until success or exhaustion.
- `RETURNING *` only captures the **first** row of the response.
- The `callback.*` shorthand is implicitly scoped to the current resource's `.iql` file.  Use the fully-qualified `{resource_name}.callback.*` form in downstream resources.

## Query options

Query options are used with query anchors to provide options for the execution of the query.

### `retries` and `retry_delay`

The `retries` and `retry_delay` query options are typically used for asynchronous or long running provider operations.  This will allow the resource time to become available or reach the desired state without failing the stack.

```sql
/*+ statecheck, retries=5, retry_delay=5 */
SELECT COUNT(*) as count FROM azure.resources.resource_groups
WHERE subscriptionId = '{{ subscription_id }}'
AND resourceGroupName = '{{ resource_group_name }}'
AND location = '{{ location }}'
AND JSON_EXTRACT(properties, '$.provisioningState') = 'Succeeded'
```

### `postdelete_retries` and `postdelete_retry_delay`

The `postdelete_retries` and `postdelete_retry_delay` query options are used in `exists` queries and are implemeneted specifically for `teardown` operations, allowing time for the resource to be deleted by the provider.

```sql
/*+ exists, postdelete_retries=10, postdelete_retry_delay=5 */
SELECT COUNT(*) as count FROM google.compute.instances
WHERE name = '{{ instance_name }}'
AND project = '{{ project }}'
AND zone = '{{ zone }}'
```

## Template Filters

StackQL Deploy uses a Jinja2-compatible templating engine and extends it with custom filters for infrastructure provisioning. For a complete reference of all available filters, see the [__Template Filters__](template-filters) documentation.

Here are a few commonly used filters:

- `from_json` - Converts JSON strings to native objects for iteration and manipulation
- `tojson` - Converts objects back to JSON strings
- `sql_escape` - Properly escapes SQL string literals for nested SQL statements
- `generate_patch_document` - Creates RFC6902-compliant patch documents for AWS resources
- `base64_encode` - Encodes strings as base64 for API fields requiring binary data

## Examples

### `resource` type example

This example is a `resource` file for a public IP address in a Google stack.

<File name='public_address.iql'>

```sql
/*+ exists */
SELECT COUNT(*) as count FROM google.compute.addresses
WHERE name = '{{ address_name }}'
AND project = '{{ project }}'
AND region = '{{ region }}'

/*+ create */
INSERT INTO google.compute.addresses
(
 project,
 region,
 data__name
) 
SELECT
'{{ project }}',
'{{ region }}',
'{{ address_name }}'

/*+ statecheck, retries=5, retry_delay=10 */
SELECT COUNT(*) as count FROM google.compute.addresses
WHERE name = '{{ address_name }}'
AND project = '{{ project }}'
AND region = '{{ region }}'

/*+ delete */
DELETE FROM google.compute.addresses
WHERE address = '{{ address_name }}' AND project = '{{ project }}'
AND region = '{{ region }}'

/*+ exports */
SELECT address
FROM google.compute.addresses
WHERE name = '{{ address_name }}'
AND project = '{{ project }}'
AND region = '{{ region }}'
```

</File>

### AWS Cloud Control API example with `callback`

This example shows a `resource` file for an S3 bucket using the AWS Cloud Control API (`awscc`), which uses an asynchronous mutation model.  The `RETURNING *` clause captures the `ProgressEvent` tracking handle and the `callback:create` / `callback:delete` blocks poll until the operation completes.

<File name='aws/s3/buckets.iql'>

```sql
/*+ exists */
SELECT COUNT(*) as count
FROM awscc.s3.buckets
WHERE region = '{{ region }}'
AND Identifier = '{{ bucket_name }}'

/*+ create */
INSERT INTO awscc.s3.buckets(BucketName, region)
SELECT '{{ bucket_name }}', '{{ region }}'
RETURNING *

/*+ callback:create, retries=10, retry_delay=15, short_circuit_field=ProgressEvent.OperationStatus, short_circuit_value=SUCCESS */
SELECT OperationStatus = 'SUCCESS' as success
FROM awscc.cloudcontrol.resource_request_statuses
WHERE region = '{{ region }}'
AND RequestToken = '{{ callback.ProgressEvent.RequestToken }}'

/*+ statecheck, retries=3, retry_delay=5 */
SELECT COUNT(*) as count
FROM awscc.s3.buckets
WHERE region = '{{ region }}'
AND Identifier = '{{ bucket_name }}'

/*+ exports */
SELECT '{{ bucket_name }}' as bucket_name

/*+ delete */
DELETE FROM awscc.s3.buckets
WHERE region = '{{ region }}'
AND Identifier = '{{ bucket_name }}'
RETURNING *

/*+ callback:delete, retries=10, retry_delay=15, short_circuit_field=ProgressEvent.OperationStatus, short_circuit_value=SUCCESS */
SELECT OperationStatus = 'SUCCESS' as success
FROM awscc.cloudcontrol.resource_request_statuses
WHERE region = '{{ region }}'
AND RequestToken = '{{ callback.ProgressEvent.RequestToken }}'
```

</File>

The corresponding manifest entry requires **no** `callback` section — callback behaviour is configured entirely in the `.iql` file:

```yaml
  - name: aws_s3_workspace_bucket
    file: aws/s3/buckets.iql
    props:
      - name: bucket_name
        value: "{{ stack_name }}-{{ stack_env }}-root-bucket"
      - name: region
        value: "ap-southeast-2"
    exports:
      - bucket_name
```

### `query` type example

This `query` example demonstrates retrieving the KMS key id for a given key alias in AWS.

<File name='get_logging_kms_key_id.iql'>

```sql
/*+ exports, retries=5, retry_delay=5 */
SELECT
target_key_id as logging_kms_key_id
FROM aws.kms.aliases
WHERE region = '{{ region }}'
AND data__Identifier = 'alias/{{ stack_name }}/{{ stack_env }}/logging';
```

</File>