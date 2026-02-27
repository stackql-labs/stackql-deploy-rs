# Exports

Exports allow resources to publish values (e.g. IDs, ARNs, names) so that
subsequent resources in the stack can reference them.

## Defining exports

Add an `exports` field to a resource in `stackql_manifest.yml`. The exports
query in the resource's `.iql` file must return columns that match the
export names.

### Simple exports

```yaml
resources:
  - name: example_vpc
    props:
      - name: cidr_block
        value: "10.0.0.0/16"
    exports:
      - vpc_id
      - vpc_cidr_block
```

The `exports` anchor in the `.iql` file must return these columns:

```sql
/*+ exports */
SELECT vpc_id, cidr_block AS vpc_cidr_block
FROM awscc.ec2.vpcs
WHERE region = '{{ region }}' AND vpc_id = '{{ vpc_id }}';
```

### Aliased exports

Use the mapping format to rename columns on export:

```yaml
exports:
  - arn: aws_iam_cross_account_role_arn
  - role_name: aws_iam_role_name
```

Here the exports query returns columns `arn` and `role_name`, but they are
stored in the context under the alias names `aws_iam_cross_account_role_arn`
and `aws_iam_role_name`.

## Referencing exported values

Exported values are injected into the global template context and can be
referenced by any subsequent resource using `{{ variable_name }}`.

### Unscoped references

The simplest way to reference an export is by its name (or alias):

```yaml
- name: example_subnet
  props:
    - name: vpc_id
      value: "{{ vpc_id }}"
```

Unscoped names follow **last-writer-wins** semantics: if two resources both
export a variable called `vpc_id`, subsequent resources will see the value
from whichever resource was processed last.

### Resource-scoped references

Every export is also available under a **resource-scoped** name of the form
`resource_name.variable_name`. This name is **immutable** -- once set, it
cannot be overwritten by a later resource:

```yaml
- name: example_subnet
  props:
    - name: vpc_id
      value: "{{ example_vpc.vpc_id }}"
```

Resource-scoped names are useful when:

- Multiple resources export variables with the same name and you need to
  reference a specific one unambiguously.
- You want to make it clear which resource a value originates from for
  readability and maintainability.

### Example

```yaml
resources:
  - name: aws_cross_account_role
    file: aws/iam/roles.iql
    props:
      - name: role_name
        value: "{{ stack_name }}-{{ stack_env }}-role"
      # ...
    exports:
      - role_name: aws_iam_cross_account_role_name
      - arn: aws_iam_cross_account_role_arn

  - name: databricks_account/credentials
    props:
      - name: credentials_name
        value: "{{ stack_name }}-{{ stack_env }}-credentials"
      - name: aws_credentials
        value:
          sts_role:
            # Unscoped -- works because only one resource exports this name
            role_arn: "{{ aws_iam_cross_account_role_arn }}"
            # Resource-scoped -- always unambiguous
            # role_arn: "{{ aws_cross_account_role.aws_iam_cross_account_role_arn }}"
```

## Protected exports

Sensitive values can be masked in log output by listing them under
`protected`:

```yaml
- name: secret_resource
  props: []
  exports:
    - api_key
  protected:
    - api_key
```

The actual value is still stored in the context and usable by templates;
only the log messages are masked.

## Stack-level exports

The top-level `exports` field in the manifest lists variables that are
written to a JSON output file (when `--output` is specified):

```yaml
exports:
  - vpc_id
  - subnet_id
```

The output file always includes `stack_name`, `stack_env`, and
`elapsed_time` automatically.
