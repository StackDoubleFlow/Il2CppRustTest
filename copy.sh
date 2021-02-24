#!/bin/bash

cargo ndk build

adb push target/aarch64-linux-android/debug/librust_test.so /sdcard/Android/data/com.beatgames.beatsaber/files/mods/
adb shell am force-stop com.beatgames.beatsaber
adb shell am start com.beatgames.beatsaber/com.unity3d.player.UnityPlayerActivity

adb logcat -c && adb logcat > test.log