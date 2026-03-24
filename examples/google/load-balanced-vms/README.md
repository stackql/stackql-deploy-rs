# example `stackql-deploy` stack

Based upon the [__terraform-google-load-balanced-vms__](https://github.com/GoogleCloudPlatform/terraform-google-load-balanced-vms) project.

![load balanced vms](https://raw.githubusercontent.com/GoogleCloudPlatform/terraform-google-load-balanced-vms/c3e9669856df44a7b7399a7119eda3ae9ce5a2fa/assets/load_balanced_vms_v1.svg)

## about `stackql-deploy`

[`stackql-deploy`](https://crates.io/crates/stackql-deploy) is a multi cloud deployment automation and testing framework which is an alternative to Terraform or similar IaC tools.  `stackql-deploy` uses a declarative model/ELT based approach to cloud resource deployment (inspired by [`dbt`](https://www.getdbt.com/)).  Advantages of `stackql-deploy` include:

- declarative framework
- no state file (state is determined from the target environment)
- multi-cloud/omni-cloud ready
- includes resource tests which can include secure config tests

## instaling `stackql-deploy`

`stackql-deploy` is installed as a python based CLI using...

```bash
install stackql-deploy from https://github.com/stackql/stackql-deploy-rs/releases
# or
pip3 install stackql-deploy
```
