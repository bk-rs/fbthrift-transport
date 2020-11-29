## fbthrift

### Build on Ubuntu 20.04


```
sudo apt install -y cmake
sudo apt install -y build-essential
```

### fmt

```
cd ~
git clone https://github.com/fmtlib/fmt.git && cd fmt
mkdir _build && cd _build
cmake ..
make -j$(nproc)
sudo make install
cd
```

### folly

```
sudo apt install -y libboost-all-dev
sudo apt install -y libssl-dev
sudo apt install -y libdouble-conversion-dev
sudo apt install -y libgoogle-glog-dev
```

```
cd ~
git clone https://github.com/facebook/folly.git && cd folly
mkdir _build && cd _build
cmake ..
make -j $(nproc)
sudo make install
cd
```

### fizz

```
sudo apt install -y libsodium-dev
sudo apt install -y libgmock-dev
```

```
cd ~
git clone https://github.com/facebookincubator/fizz && cd fizz/fizz
mkdir _build && cd _build
cmake ..
make -j $(nproc)
sudo make install
cd
```

### wangle

```
cd ~
git clone https://github.com/facebook/wangle.git && cd wangle/wangle
mkdir _build && cd _build
cmake ..
make -j $(nproc)
sudo make install
cd
```

### fbthrift

```
sudo apt install -y libbison-dev
sudo apt install -y libfl-dev
sudo apt install -y libzstd-dev
```

```
cd ~
git clone https://github.com/facebook/fbthrift && cd fbthrift
git checkout c18333c
mkdir _build && cd _build
cmake ..
make -j $(nproc)
sudo make install
which thrift1
cd
```
