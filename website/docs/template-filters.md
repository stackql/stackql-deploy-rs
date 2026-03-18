---
id: template-filters
title: Template Filters
hide_title: false
hide_table_of_contents: false
description: Custom and built-in template filters available in StackQL Deploy for template processing
tags: []
draft: false
unlisted: false
---

import File from '/src/components/File';

# Template Filters

StackQL Deploy uses a Jinja2-compatible templating engine ([Tera](https://keats.github.io/tera/)) and extends it with custom filters specifically designed for infrastructure provisioning use cases. These filters help transform data between formats, encode values, generate specialized document formats, and perform other common operations required in IaC configurations.

## Available Filters

### `from_json`

Converts a JSON string to a native object (dictionary or list). This is commonly used to enable iteration over complex data structures in templates.

**Example usage:**

```sql
{% for network_interface in network_interfaces | from_json %}
INSERT INTO google.compute.instances 
 (
  /* fields... */
 ) 
 SELECT
'{{ instance_name_prefix }}-{{ loop.index }}',
/* other values... */
'[ {{ network_interface | tojson }} ]';
{% endfor %}
```

### `tojson`

A built-in filter that converts a dictionary or list into a JSON string. Often used in conjunction with `from_json` when working with complex data structures.

**Example usage:**

```sql
'[ {{ network_interface | tojson }} ]'
```

### `generate_patch_document`

Generates a patch document according to [RFC6902](https://datatracker.ietf.org/doc/html/rfc6902), primarily designed for the AWS Cloud Control API which requires patch documents for resource updates.

**Example usage:**

```sql
update aws.s3.buckets 
set PatchDocument = string('{{ {
    "NotificationConfiguration": transfer_notification_config
    } | generate_patch_document }}') 
WHERE 
region = '{{ region }}' 
AND Identifier = '{{ bucket_name }}';
```

### `base64_encode`

Encodes a string as base64, which is commonly required for certain API fields that accept binary data.

**Example usage:**

```sql
INSERT INTO aws.ec2.instances (
 /* fields... */
 UserData,
 region
)
SELECT 
 /* values... */
 '{{ user_data | base64_encode }}',
 '{{ region }}';
```

### `sql_list`

Converts a list or a JSON array string into a SQL-compatible list format with proper quoting, suitable for use in SQL IN clauses.

**Example usage:**

```sql
SELECT * FROM aws.ec2.instances
WHERE region = '{{ region }}'
AND InstanceId IN {{ instance_ids | sql_list }}
```

### `sql_escape`

Escapes a string for use as a SQL string literal by doubling any single quotes. This is particularly useful for nested SQL statements where quotes need special handling.

**Example usage:**

```sql
INSERT INTO snowflake.sqlapi.statements (
statement,
/* other fields... */
)
SELECT 
'{{ statement | sql_escape }}',
/* other values... */
;
```

### `merge_lists`

Merges two lists (or JSON-encoded list strings) into a single list with unique items.

**Example usage:**

```sql
{% set combined_policies = default_policies | merge_lists(custom_policies) %}
INSERT INTO aws.iam.policies (
  /* fields... */
  PolicyDocument,
  /* other fields... */
)
SELECT
  /* values... */
  '{{ combined_policies | tojson }}',
  /* other values... */
;
```

### `merge_objects`

Merges two dictionaries (or JSON-encoded object strings) into a single dictionary. In case of duplicate keys, values from the second dictionary take precedence.

**Example usage:**

```sql
{% set complete_config = base_config | merge_objects(environment_specific_config) %}
INSERT INTO aws.lambda.functions (
  /* fields... */
  Environment,
  /* other fields... */
)
SELECT
  /* values... */
  '{{ complete_config | tojson }}',
  /* other values... */
;
```

### `to_aws_tag_filters`

Converts a list of AWS tag key-value pairs (as used in `global_tags`) into the AWS Resource Groups Tagging API `TagFilters` format. This is an AWS-specific filter designed for use with `awscc.tagging.tagged_resources` queries.

**Input format:** `[{"Key": "k", "Value": "v"}, ...]`
**Output format:** `[{"Key": "k", "Values": ["v"]}, ...]`

**Example usage:**

```sql
/*+ exists */
SELECT split_part(ResourceARN, '/', 2) as vpc_id
FROM awscc.tagging.tagged_resources
WHERE region = '{{ region }}'
AND TagFilters = '{{ global_tags | to_aws_tag_filters }}'
AND ResourceTypeFilters = '["ec2:vpc"]'
```

This filter is typically applied to the `global_tags` variable defined in the manifest:

```yaml
globals:
  - name: global_tags
    value:
      - Key: 'stackql:stack-name'
        Value: "{{ stack_name }}"
      - Key: 'stackql:stack-env'
        Value: "{{ stack_env }}"
      - Key: 'stackql:resource-name'
        Value: "{{ resource_name }}"
```

## Special Variables

StackQL Deploy injects the following built-in variables automatically — no manifest configuration is required.

### `stack_name`

The name of the stack as declared in `stackql_manifest.yml`.  Available in every template context.

```sql
INSERT INTO google.compute.networks (project, name)
SELECT '{{ project }}', '{{ stack_name }}-{{ stack_env }}-vpc'
```

### `stack_env`

The environment name supplied to the CLI (e.g. `dev`, `sit`, `prd`).  Available in every template context.

### `resource_name`

The name of the resource currently being processed.  Available in every resource template context.

```sql
/*+ create */
INSERT INTO google.logging.sinks (parent, name)
SELECT 'projects/{{ project }}', '{{ resource_name }}-sink'
```

### `idempotency_token`

A UUID v4 that is generated **once per resource per session (invocation)** and remains stable for the lifetime of that run.  This is particularly important for asynchronous mutation operations where a provider needs to reliably distinguish a genuine new request from a retry of an earlier request.

| Access form | Where available |
|---|---|
| `{{ idempotency_token }}` | Inside the resource's own `.iql` file |
| `{{ this.idempotency_token }}` | Inside the resource's own `.iql` file (preferred, explicit) |
| `{{ <resource_name>.idempotency_token }}` | In any downstream resource template |

**Example — passing a client token to AWS Cloud Control API:**

```sql
/*+ create */
INSERT INTO awscc.cloudformation.stacks(
  StackName,
  TemplateURL,
  ClientRequestToken,
  region
)
SELECT
  '{{ stack_name }}-{{ stack_env }}',
  '{{ template_url }}',
  '{{ this.idempotency_token }}',
  '{{ region }}'
RETURNING *
```

**Example — referencing another resource's token from a downstream resource:**

```sql
/*+ create */
INSERT INTO awscc.some.resource(ParentToken, region)
SELECT '{{ my_upstream_resource.idempotency_token }}', '{{ region }}'
```

:::note

`{{ uuid() }}` (see below) generates a **new** UUID on every template render, so retrying the same query produces a different value each time.  Use `{{ this.idempotency_token }}` instead when you need a stable, retry-safe identifier.

:::

## Global Functions

### `uuid`

Generates a random UUID (version 4). Useful for creating unique identifiers.

**Example usage:**

```sql
INSERT INTO aws.s3.buckets (
  /* fields... */
  BucketName,
  /* other fields... */
)
SELECT
  /* values... */
  '{{ stack_name }}-{{ uuid() }}',
  /* other values... */
;
```

## Filter Chaining

Filters can be chained together to perform multiple transformations in sequence:

```sql
'{{ user_config | from_json | merge_objects(default_config) | tojson | base64_encode }}'
```

## Custom Filter Development

The StackQL Deploy filtering system is extensible. If you need additional filters for your specific use case, you can contribute to the project by adding new filters in the [stackql-deploy-rs](https://github.com/stackql-labs/stackql-deploy-rs) repository.