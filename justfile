DEFAULT_PLATFORM := "linux"
GODOT_COMMAND := "flatpak run --command='/home/applemango/.var/app/io.github.MakovWait.Godots/data/godot/app_userdata/Godots/versions/Godot_v4_6_3-stable_linux_x86_64/Godot_v4.6.3-stable_linux.x86_64' io.github.MakovWait.Godots"

default: build
set shell := ["sh", "-cux"]

[arg("ARGS", help="Used to sprcify specific tests to be run.")]
test *ARGS="":
    cargo test --manifest-path mines_core/Cargo.toml --features godot {{ ARGS }}

edit:
    {{ GODOT_COMMAND }} --path ./game -e &> /dev/null &

[arg("MODE", pattern="debug|release")]
[arg("PLATFORM", pattern="linux|android", help="A valid rustc target platform.")]
[script]
build PLATFORM=DEFAULT_PLATFORM MODE="debug": test
    if [ "{{ PLATFORM }}" = "android" ]; then
        env PLATFORM="aarch64-linux-android" just build-android "{{ MODE }}"
    elif [ "{{ PLATFORM }}" = "linux" ] || [ "{{ PLATFORM }}" = "linux_x86_64" ]; then
        env PLATFORM="x86_64-unknown-linux-gnu" just build-linux_x86_64 "{{ MODE }}"
    fi

android_ndk_version := "25.2.9519653"
android_ndk_home := env("ANDROID_NDK_HOME", env("HOME") + "/Android/Sdk/ndk/" + android_ndk_version)
android_llvm_path := android_ndk_home + "/toolchains/llvm/prebuilt/linux-x86_64/bin"

[arg("MODE", pattern="debug|release")]
[private]
[script]
build-android MODE="debug":
    rustup target add "$PLATFORM"
    export CLANG_PATH="{{ android_llvm_path }}/clang"
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="{{ android_llvm_path }}/aarch64-linux-android33-clang"
    cargo build {{ if MODE == "release" { "--release" } else { "" } }} \
        --manifest-path mines_core/Cargo.toml \
        --features godot \
        --target="$PLATFORM"

    mkdir -p "./build/android"
    {{ GODOT_COMMAND }} --headless --path "./game" {{ if MODE == "release" { "--export-release" } else { "--export-debug" } }} "Android" "../build/android/release.apk"

[arg("MODE", pattern="debug|release")]
[private]
[script]
build-linux_x86_64 MODE="debug":
    rustup target add "$PLATFORM"
    cargo build {{ if MODE == "release" { "--release" } else { "" } }} \
        --manifest-path mines_core/Cargo.toml \
        --features godot \
        --target="$PLATFORM"

    mkdir -p "./build/linux_x86_64"
    {{ GODOT_COMMAND }} --headless --path "./game" {{ if MODE == "release" { "--export-release" } else { "--export-debug" } }} "Linux x86_64" "../build/linux_x86_64/release.zip"

release PLATFORM=DEFAULT_PLATFORM:
    just build {{ PLATFORM }} release
