rm -rf stackql-deploy
rm -rf stackql
rm -rf stackql-aws-cloud-shell.sh
rm -rf stackql-azure-cloud-shell.sh
rm -rf stackql-google-cloud-shell.sh
rm -rf stackql-databricks-shell.sh
cargo build --release
cp target/release/stackql-deploy stackql-deploy
./stackql-deploy upgrade