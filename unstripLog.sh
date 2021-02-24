#!/bin/bash

cat ./test.log | $ANDROID_NDK_HOME/ndk-stack -sym ./target/aarch64-linux-android/debug > test_unstripped.log