# example `stackql-deploy` stack

Based upon the [Kubernetes the Hard Way](https://github.com/kelseyhightower/kubernetes-the-hard-way) project.

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
