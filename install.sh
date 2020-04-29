#! /bin/bash

cargo build --release
retVal=$?
if [ $retVal -eq 0 ]; then
    sudo cp ./target/release/tisk /usr/local/bin/
fi
exit $retVal
