#!/usr/bin/env just --justfile

# just is "just a command runner."
# Docs: https://just.systems/man/en/chapter_1.html

FPM_COMMAND_BASE := '	  -p ./target/ftb-rs-0.1.0-1-x86_64.rpm \
	  --name ftb-rs \
	  --license mit \
	  --version 0.1.0 \
	  --architecture x86_64 \
	  --description "Generates and uploads a tilesheet to FTBWiki from an icon dump." \
	  --url "https://github.com/FTB-Gamepedia/ftb-rs/" \
	  --maintainer "retep998 <retep998@gmail.com>"
'
install-target TARGET:
	rustup target add {{ TARGET }}
	#rustup target add x86_64-pc-windows-gnu
	#rustup target add x86_64-apple-darwin

build TARGET: (install-target TARGET)
	cargo build --release --target={{ TARGET }}
	#cargo build --release --target=x86_64-unknown-linux-gnu
	#cargo build --release --target=x86_64-pc-windows-gnu
	#cargo build --release --target=x86_64-apple-darwin

build-linux: (build "x86_64-unknown-linux-gnu")
	true

build-windows: (build "x86_64-pc-windows-gnu")
	true

package-linux-deb: (build-linux)
	fpm \
	  -s dir -t rpm \
	  -p ./target/ftb-rs-0.1.0-1-x86_64.deb
	  {{ FPM_COMMAND_BASE }} \
	  ./target/x86_64-unknown-linux-gnu/release/ftb=/usr/bin/ftb-rs

package-linux-rpm: (build-linux)
	fpm \
	  -s dir -t rpm \
	  -p ./target/ftb-rs-0.1.0-1-x86_64.rpm \
	  {{ FPM_COMMAND_BASE }} \
	  ./target/x86_64-unknown-linux-gnu/release/ftb=/usr/bin/ftb-rs

package-linux-pacman: (build-linux)
	fpm \
	  -s dir -t pacman \
	  -p ./target/ftb-rs-0.1.0-1-x86_64.pacman \
	  {{ FPM_COMMAND_BASE }} \
	  ./target/x86_64-unknown-linux-gnu/release/ftb=/usr/bin/ftb-rs

setup-macos: (install-target "x86_64-apple-darwin")
	#!/bin/bash
	-git clone https://github.com/tpoechtrager/osxcross
	cd osxcross
	wget -nc https://s3.dockerproject.org/darwin/v2/MacOSX10.10.sdk.tar.xz
	mkdir tarballs/
	mv MacOSX10.10.sdk.tar.xz tarballs/
	UNATTENDED=yes OSX_VERSION_MIN=10.7 ./build.sh

build-macos: setup-macos
	#!/bin/bash
	MACOS_TARGET="x86_64-apple-darwin"

	echo "Building target for platform ${MACOS_TARGET}"
	echo

	# Add osxcross toolchain to path
	export PATH="$(pwd)/osxcross/target/bin:$PATH"

	# Make libz-sys (git2-rs -> libgit2-sys -> libz-sys) build as a statically linked lib
	# This prevents the host zlib from being linked
	export LIBZ_SYS_STATIC=1

	# Use Clang for C/C++ builds
	export CC=o64-clang
	export CXX=o64-clang++

	cargo build --release --target "${MACOS_TARGET}"
