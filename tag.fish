#!/usr/bin/env fish
git tag -m "Release" (yj -- -tj < Cargo.toml | jq -r .package.version)
