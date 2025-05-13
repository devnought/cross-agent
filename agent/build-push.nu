cross build --release --target aarch64-linux-android
adb push ../target/aarch64-linux-android/release/agent /data/local/tmp/agent
adb shell "chmod 775 /data/local/tmp/agent"
