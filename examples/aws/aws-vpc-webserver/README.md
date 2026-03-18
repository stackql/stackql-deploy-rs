# AWS VPC Web Server Example

This example provisions a complete AWS networking stack with an Apache web server using the `awscc` (Cloud Control) provider exclusively.

## Architecture

```mermaid
architecture-beta
    group vpc(logos:aws-vpc)[VPC 10.x.0.0/16]

    service subnet(logos:aws-vpc)[Subnet 10.x.1.0/24] in vpc
    service rt(logos:aws-route-53)[Route Table] in vpc
    service sg(logos:aws-shield)[Security Group] in vpc
    service ec2(logos:aws-ec2)[Web Server t2.micro] in vpc

    group edge(logos:aws-cloudfront)[Edge]

    service igw(logos:aws-api-gateway)[Internet Gateway] in edge

    igw:R --> L:rt
    rt:B -- T:subnet
    sg:R -- L:ec2
    subnet:T -- B:ec2
```

## Resources

| # | Resource | Provider Resource | Description |
|---|----------|-------------------|-------------|
| 1 | `example_vpc` | `awscc.ec2.vpcs` | VPC with DNS support and hostnames enabled |
| 2 | `example_subnet` | `awscc.ec2.subnets` | Public subnet with auto-assign public IP |
| 3 | `example_inet_gateway` | `awscc.ec2.internet_gateways` | Internet gateway for outbound/inbound traffic |
| 4 | `example_inet_gw_attachment` | `awscc.ec2.vpc_gateway_attachments` | Attaches IGW to VPC |
| 5 | `example_route_table` | `awscc.ec2.route_tables` | Custom route table for the VPC |
| 6 | `example_subnet_rt_assn` | `awscc.ec2.subnet_route_table_associations` | Associates subnet with route table |
| 7 | `example_inet_route` | `awscc.ec2.routes` | Default route (0.0.0.0/0) to internet gateway |
| 8 | `example_security_group` | `awscc.ec2.security_groups` | Allows HTTP (80) from anywhere, SSH (22) from VPC CIDR |
| 9 | `example_web_server` | `awscc.ec2.instances` | t2.micro running Apache with a landing page |
| 10 | `get_web_server_url` | `awscc.ec2.instances` | Retrieves the public DNS name of the instance |

## Environment-Specific CIDR Blocks

| Environment | VPC CIDR | Subnet CIDR |
|-------------|----------|-------------|
| `prd` | 10.0.0.0/16 | 10.0.1.0/24 |
| `sit` | 10.1.0.0/16 | 10.1.1.0/24 |
| `dev` | 10.2.0.0/16 | 10.2.1.0/24 |

## Prerequisites

- `stackql-deploy` installed ([releases](https://github.com/stackql/stackql-deploy-rs/releases))
- AWS credentials set as environment variables:

  ```bash
  export AWS_ACCESS_KEY_ID=your_access_key
  export AWS_SECRET_ACCESS_KEY=your_secret_key
  export AWS_REGION=us-east-1
  ```

## Usage

### Deploy

```bash
stackql-deploy build examples/aws/aws-vpc-webserver dev
```

With query visibility:

```bash
stackql-deploy build examples/aws/aws-vpc-webserver dev --show-queries
```

Dry run (no changes):

```bash
stackql-deploy build examples/aws/aws-vpc-webserver dev --dry-run --show-queries
```

### Test

```bash
stackql-deploy test examples/aws/aws-vpc-webserver dev
```

### Teardown

```bash
stackql-deploy teardown examples/aws/aws-vpc-webserver dev
```

### Debug mode

```bash
stackql-deploy build examples/aws/aws-vpc-webserver dev --log-level debug
```

## How It Works

### Tag Taxonomy

All taggable resources are tagged with three keys used for identification:

| Tag Key | Value | Purpose |
|---------|-------|---------|
| `stackql:stack-name` | `{{ stack_name }}` | Identifies the stack |
| `stackql:stack-env` | `{{ stack_env }}` | Identifies the deployment environment |
| `stackql:resource-name` | `{{ resource_name }}` | Identifies the specific resource |

These are defined once as `global_tags` in the manifest and merged into each resource's tags.

### Exists Query Pattern

The `exists` query uses a CTE that cross-references `awscc.tagging.tagged_resources` with the provider's `*_list_only` resource to confirm the resource actually exists (not just a stale tag record):

```sql
/*+ exists */
WITH tagged_resources AS (
    SELECT split_part(ResourceARN, '/', 2) as vpc_id
    FROM awscc.tagging.tagged_resources
    WHERE region = '{{ region }}'
    AND TagFilters = '{{ global_tags | to_aws_tag_filters }}'
    AND ResourceTypeFilters = '["ec2:vpc"]'
),
vpcs AS (
    SELECT vpc_id FROM awscc.ec2.vpcs_list_only
    WHERE region = '{{ region }}'
)
SELECT r.vpc_id
FROM vpcs r
INNER JOIN tagged_resources tr ON r.vpc_id = tr.vpc_id;
```

The returned field (e.g. `vpc_id`) is automatically captured as `this.vpc_id` and made available to all subsequent queries for that resource.

### `to_aws_tag_filters` Filter

The `global_tags` variable (a list of `Key`/`Value` pairs) is converted to the AWS TagFilters format using the `to_aws_tag_filters` custom Tera filter:

```
{{ global_tags | to_aws_tag_filters }}
```

Transforms `[{"Key":"k","Value":"v"}]` into `[{"Key":"k","Values":["v"]}]`.

### Statecheck Pattern

The `statecheck` query uses `this.<field>` (captured from exists) to query the actual resource via Cloud Control and verify properties match the desired state, including tag comparison using `AWS_POLICY_EQUAL`:

```sql
/*+ statecheck, retries=5, retry_delay=5 */
SELECT COUNT(*) as count FROM (
    SELECT AWS_POLICY_EQUAL(tags, '{{ vpc_tags }}') as test_tags
    FROM awscc.ec2.vpcs
    WHERE Identifier = '{{ this.vpc_id }}'
    AND region = '{{ region }}'
    AND cidr_block = '{{ vpc_cidr_block }}'
) t
WHERE test_tags = 1;
```

### Non-Taggable Resources

Resources that don't support tags use alternative patterns:

- **VPC Gateway Attachment**: `count`-based exists using `Identifier = 'IGW|{{ vpc_id }}'`
- **Subnet Route Table Association**: exists via `vw_subnet_route_table_associations` view, field captured as `this.subnet_route_table_assn_id`
- **Route**: `createorupdate` pattern (always attempts insert)

### Troubleshooting

Check failed Cloud Control requests:

```sql
SELECT * FROM awscc.cloud_control.resource_requests
WHERE ResourceRequestStatusFilter = '{"OperationStatuses": ["FAILED"], "Operations": ["CREATE"]}'
AND region = 'us-east-1';
```
