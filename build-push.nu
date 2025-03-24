cross build --release --target aarch64-linux-android
adb push .\target\aarch64-linux-android\release\android-test /data/local/tmp/android-test
adb shell "chmod 775 /data/local/tmp/android-test"
