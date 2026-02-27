# Basic build (debug mode)
cargo build

# Release build (optimized with no debug info)
cargo build --release

# Build with verbose output
cargo build -v

# Check if your code compiles without producing an executable
cargo check

# Build and run the application
cargo run

# Build and run with command line arguments
cargo run -- build --env prod --provider aws --region us-east-1

./target/release/stackql-deploy --version

./target/release/stackql-deploy --help

./target/release/stackql-deploy info

./target/release/stackql-deploy init my-stack --provider aws

./target/release/stackql-deploy build my-stack dev

./target/release/stackql-deploy test my-stack dev

./target/release/stackql-deploy test examples/aws/aws-stack dev

./target/release/stackql-deploy teardown my-stack dev

./target/release/stackql-deploy build

./target/release/stackql-deploy unknowncmd

./target/release/stackql-deploy shell

./target/release/stackql-deploy upgrade

./target/release/stackql-deploy start-server

# Using built-in provider template
./target/release/stackql-deploy init my-project --provider aws

# Using relative path to template in GitHub
./target/release/stackql-deploy init my-project --template google/starter

# Using full GitHub URL
./target/release/stackql-deploy init my-project --template https://github.com/stackql/stackql-deploy-rust/tree/main/template-hub/azure/starter

./target/release/stackql-deploy init my-project --template https://raw.githubusercontent.com/stackql/stackql-deploy-rust/main/template-hub/azure/starter

./target/release/stackql-deploy init my-project --template https://raw.githubusercontent.com/stackql/stackql-deploy-rust/main/template-hub/azure/fred


#### test

git fetch origin && git merge origin/main

cargo build --release

./target/release/stackql-deploy build \
examples/databricks/serverless dev \
-e AWS_REGION=${AWS_REGION} \
-e AWS_ACCOUNT_ID=${AWS_ACCOUNT_ID} \
-e DATABRICKS_ACCOUNT_ID=${DATABRICKS_ACCOUNT_ID} \
-e DATABRICKS_AWS_ACCOUNT_ID=${DATABRICKS_AWS_ACCOUNT_ID} \
--dry-run

pgrep -f "stackql srv"
kill $(pgrep -f "stackql srv")