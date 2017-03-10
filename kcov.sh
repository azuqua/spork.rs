#!/bin/bash

# sudo apt-get install libcurl4-openssl-dev libelf-dev libdw-dev libbfd-dev libiberty-dev

if [ ! -d kcov-33 ]
  then
    pushd .
    cd tmp

    # latest as of Jan 2017
    wget https://github.com/SimonKagstrom/kcov/archive/v33.tar.gz
    tar xzf v33.tar.gz && mkdir kcov-33/build 
    cd kcov-33/build
    cmake -DCMAKE_INSTALL_PREFIX:PATH=../release ..
    make -j2
    make install
    popd
fi

cargo clean -p spork

# https://stackoverflow.com/a/38371687/2966874
RUSTFLAGS='-C link-dead-code' cargo test --no-run
tmp/kcov-33/release/bin/kcov --verify --exclude-pattern=.cargo --exclude-path=/usr/lib target/kcov \
  target/debug/spork-* && (google-chrome file://$PWD/target/kcov/index.html > /dev/null 2>&1 &)

# rm -rf tmp/*

echo "Finished generating test coverage reports in target/kcov."