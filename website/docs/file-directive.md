---
id: file-directive
title: file() Directive
hide_title: false
hide_table_of_contents: false
description: Use the file() directive to modularize manifest values by including external JSON or YAML files
tags: []
draft: false
unlisted: false
---

import File from '/src/components/File';

The `file()` directive allows you to include the contents of external JSON or YAML files directly into your `stackql_manifest.yml` property values. This is particularly useful for modularizing large or reusable configuration blocks like IAM policy statements, role definitions, or any complex nested structures.

## Syntax

```yaml
file(relative/path/to/file.json)
```

The argument is a file path **relative to the `resources/` directory** of your stack (the same base path used for `.iql` resource query files).

## Supported file formats

| Extension | Format |
|---|---|
| `.json` | Parsed as JSON |
| `.yml`, `.yaml` | Parsed as YAML |
| Other/none | Tries JSON first, falls back to YAML |

## How it works

When the manifest is loaded, the `file()` directive is resolved **before** any template variable substitution occurs. The referenced file is read and parsed, and the resulting value (object, array, or scalar) replaces the `file()` string in the manifest value tree.

:::note

Because `file()` directives are resolved at manifest load time (before template rendering), the included files should contain static data only. Template variables like `{{ stack_name }}` in property values surrounding the `file()` directive will still be rendered as normal.

:::

## Usage

### Including individual items in a list

You can use `file()` as individual items within a YAML sequence. Each directive is replaced by the parsed contents of the referenced file.

<File name='stackql_manifest.yml'>

```yaml
resources:
  - name: aws/iam/cross_account_role
    file: aws/iam/iam_role.iql
    props:
      - name: policies
        value:
          - PolicyDocument:
              Statement:
                - file(aws/iam/policies/ec2_permissions.json)
                - file(aws/iam/policies/iam_service_linked_role.json)
              Version: '2012-10-17'
            PolicyName: "{{ stack_name }}-{{ stack_env }}-policy"
```

</File>

Where `resources/aws/iam/policies/ec2_permissions.json` contains a single policy statement object:

<File name='resources/aws/iam/policies/ec2_permissions.json'>

```json
{
  "Sid": "Stmt1403287045000",
  "Effect": "Allow",
  "Action": [
    "ec2:AllocateAddress",
    "ec2:AssociateDhcpOptions",
    "ec2:AssociateIamInstanceProfile",
    "ec2:AssociateRouteTable",
    "ec2:AttachInternetGateway",
    "ec2:AuthorizeSecurityGroupEgress",
    "ec2:AuthorizeSecurityGroupIngress",
    "ec2:CreateSecurityGroup",
    "ec2:CreateSubnet",
    "ec2:CreateTags",
    "ec2:CreateVpc",
    "ec2:DeleteSecurityGroup",
    "ec2:DeleteSubnet",
    "ec2:DeleteVpc",
    "ec2:DescribeInstances",
    "ec2:DescribeSecurityGroups",
    "ec2:DescribeSubnets",
    "ec2:DescribeVpcs",
    "ec2:RunInstances",
    "ec2:TerminateInstances"
  ],
  "Resource": ["*"]
}
```

</File>

And `resources/aws/iam/policies/iam_service_linked_role.json`:

<File name='resources/aws/iam/policies/iam_service_linked_role.json'>

```json
{
  "Effect": "Allow",
  "Action": [
    "iam:CreateServiceLinkedRole",
    "iam:PutRolePolicy"
  ],
  "Resource": [
    "arn:aws:iam::*:role/aws-service-role/spot.amazonaws.com/AWSServiceRoleForEC2Spot"
  ],
  "Condition": {
    "StringLike": {
      "iam:AWSServiceName": "spot.amazonaws.com"
    }
  }
}
```

</File>

### Replacing an entire value with a file

You can also use `file()` to replace an entire value, such as a complete list of statements:

<File name='stackql_manifest.yml'>

```yaml
resources:
  - name: aws/iam/cross_account_role
    file: aws/iam/iam_role.iql
    props:
      - name: policies
        value:
          - PolicyDocument:
              Statement: file(aws/iam/policies/cross_account_statements.json)
              Version: '2012-10-17'
            PolicyName: "{{ stack_name }}-{{ stack_env }}-policy"
```

</File>

Where `resources/aws/iam/policies/cross_account_statements.json` is a JSON **array**:

<File name='resources/aws/iam/policies/cross_account_statements.json'>

```json
[
  {
    "Sid": "Stmt1403287045000",
    "Effect": "Allow",
    "Action": ["ec2:AllocateAddress", "ec2:CreateVpc", "ec2:DeleteVpc"],
    "Resource": ["*"]
  },
  {
    "Effect": "Allow",
    "Action": ["iam:CreateServiceLinkedRole", "iam:PutRolePolicy"],
    "Resource": ["arn:aws:iam::*:role/aws-service-role/spot.amazonaws.com/AWSServiceRoleForEC2Spot"],
    "Condition": {
      "StringLike": {
        "iam:AWSServiceName": "spot.amazonaws.com"
      }
    }
  }
]
```

</File>

### Using YAML files

YAML files can be used instead of JSON, which can be more readable for some configurations:

<File name='stackql_manifest.yml'>

```yaml
resources:
  - name: aws/iam/metastore_access_role
    file: aws/iam/iam_role.iql
    props:
      - name: policies
        value:
          - PolicyName: MetastoreS3Policy
            PolicyDocument:
              Version: '2012-10-17'
              Statement: file(aws/iam/policies/metastore_statements.yaml)
```

</File>

<File name='resources/aws/iam/policies/metastore_statements.yaml'>

```yaml
- Effect: Allow
  Action:
    - "s3:GetObject"
    - "s3:PutObject"
    - "s3:DeleteObject"
    - "s3:ListBucket"
    - "s3:GetBucketLocation"
  Resource:
    - "arn:aws:s3:::my-metastore-bucket/*"
    - "arn:aws:s3:::my-metastore-bucket"
- Effect: Allow
  Action:
    - "sts:AssumeRole"
  Resource:
    - "arn:aws:iam::123456789012:role/my-metastore-role"
```

</File>

### Using in globals

The `file()` directive also works in global variable values:

<File name='stackql_manifest.yml'>

```yaml
globals:
  - name: default_tags
    value: file(common/default_tags.json)
```

</File>

### Subdirectory organization

File paths can include subdirectories, making it easy to organize included files alongside your resource query files:

```
my-stack/
  stackql_manifest.yml
  resources/
    aws/
      iam/
        iam_role.iql
        policies/
          ec2_permissions.json
          iam_service_linked_role.json
          metastore_statements.yaml
      s3/
        s3_bucket.iql
```

### Nested file() directives

Included files can themselves contain `file()` directives, which will be resolved recursively. This allows you to compose configurations from multiple reusable fragments:

<File name='resources/aws/iam/policies/combined_policy.yaml'>

```yaml
Version: '2012-10-17'
Statement:
  - file(aws/iam/policies/base_permissions.json)
  - file(aws/iam/policies/extra_permissions.json)
```

</File>

## Error handling

If a `file()` directive references a file that does not exist or contains invalid JSON/YAML, the manifest will fail to load with a descriptive error message indicating the problematic file path and the nature of the error.
