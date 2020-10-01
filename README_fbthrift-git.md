## fbthrift-git

### Release new version

```
git clone git@github.com:facebook/fbthrift.git
cd fbthrift
git checkout master
git fetch origin
git branch -D fbthrift-rust
git subtree split -P thrift/lib/rust -b fbthrift-rust
git checkout fbthrift-rust

sed -i 's/^name = "fbthrift"$/name = "fbthrift-git"/' Cargo.toml
sed -i 's/^version = "0.0.1+unstable"$/version = "0.0.2+617c2e9"/' Cargo.toml
sed -i '/^include = [.*]*/a description = "This is a -git package for fbthrift."\r\nlicense = "Apache-2.0 OR MIT"\r\nauthors = ["Facebook"]' Cargo.toml
sed -i '/^publish = false$/d'  Cargo.toml

git commit -a -m "Release 0.0.2+617c2e9"

cargo publish --dry-run
cargo publish

git reset HEAD~1
git checkout -f
```
