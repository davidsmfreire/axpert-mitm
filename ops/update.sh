#!/bin/sh

set -xe

rsync -r src pi@192.168.1.2:~/axpert-mitm
rsync Cargo.lock pi@192.168.1.2:~/axpert-mitm/Cargo.lock
rsync Cargo.toml pi@192.168.1.2:~/axpert-mitm/Cargo.toml
rsync .env pi@192.168.1.2:~/axpert-mitm/.env

ssh pi@192.168.1.2 'cd ~/axpert-mitm && cargo build --release'
ssh pi@192.168.1.2 'sudo systemctl restart axpert-mitm'
# run "sudo systemctl restart axpert-mitm"s