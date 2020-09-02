## fbthrift

### Build on Ubuntu 20.04


```
apt install cmake
apt install build-essential
```

### fmt

```
cd
git clone https://github.com/fmtlib/fmt.git && cd fmt
mkdir _build && cd _build
cmake ..
make -j$(nproc)
sudo make install
```

### folly

```
apt install libboost-all-dev
apt install libssl-dev
apt install libdouble-conversion-dev
apt install libgoogle-glog-dev
```

```
cd
git clone git@github.com:facebook/folly.git && cd folly
mkdir _build && cd _build
cmake ..
make -j $(nproc)
sudo make install
```

### fizz

```
apt install libsodium-dev
```

```
cd
git clone https://github.com/facebookincubator/fizz && cd fizz/fizz
mkdir _build && cd _build
cmake ..
make -j $(nproc)
sudo make install
```

### wangle

```
cd
git clone https://github.com/facebook/wangle.git && cd wangle/wangle
mkdir _build && cd _build
cmake ..
make -j $(nproc)
sudo make install
```

### fbthrift

```
apt install libbison-dev
apt install libfl-dev
apt install libzstd-dev
```

```
cd
git clone https://github.com/facebook/fbthrift && cd fbthrift
git checkout v2020.08.31.00
cd build
cmake ..
make -j $(nproc)
sudo make install
```
