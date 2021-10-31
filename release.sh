#!/bin/bash

cargo build --release

mkdir -p target/final-zip
rm -rf traget/final-zip/the-purple-night
cp -rv release-data/the-purple-night target/final-zip

arm-none-eabi-objcopy -O binary target/thumbv4t-none-eabi/release/minijam92 target/final-zip/the-purple-night/thepurplenight.gba
gbafix -p -tPURPLENIGHT -cPURP -mGC target/final-zip/the-purple-night/thepurplenight.gba

cp screenshot.png target/final-zip/the-purple-night

rm -f target/thepurplenight.zip
(cd target/final-zip && zip -r ../thepurplenight.zip the-purple-night)

cp -r html target/html
cp target/final-zip/the-purple-night/thepurplenight.gba target/html/thepurplenight.gba
(cd target/html && zip ../html.zip ./*)
